#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	traits::{ConstU32, EstimateNextSessionRotation, Randomness},
	BoundedVec, PalletId, WeakBoundedVec,
};
use frame_system::offchain::{
	AppCrypto, CreateSignedTransaction, SignedPayload, Signer, SigningTypes,
};
use scale_info::TypeInfo;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	app_crypto::RuntimeAppPublic,
	offchain::{
		http,
		storage::{StorageRetrievalError, StorageValueRef},
		Duration,
	},
	RuntimeDebug,
};
use sp_std::cmp::{Eq, PartialEq};

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"mail");
pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;
	// implemented for ocw-runtime
	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
enum OffchainErr {
	UnexpectedError,
	Ineligible,
	GenerateInfoError,
	NetworkState,
	FailedSigning,
	Overflow,
	Working,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Mail {
	timestamp: u64,
	store_hash: BoundedVec<u8, ConstU32<128>>,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum MailAddress {
	SubAddr(BoundedVec<u8, ConstU32<128>>), // substrate address, start with 5...
	ETHAddr(BoundedVec<u8, ConstU32<128>>), // ethereum address, start with 0...
	MoonbeamAddr(BoundedVec<u8, ConstU32<128>>), // moonbeam address, start with 0...
	NormalAddr(BoundedVec<u8, ConstU32<128>>), /* normal address, such as gmail, outlook.com...
	                                         * //1@q.cn */
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	use frame_support::pallet_prelude::*;
	use frame_system::{offchain::SendUnsignedTransaction, pallet_prelude::*};
	use sp_runtime::{Permill, SaturatedConversion};

	pub const LIMIT: u64 = u64::MAX;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		#[pallet::constant]
		type LockTime: Get<BlockNumberOf<Self>>;

		#[pallet::constant]
		type StringLimit: Get<u32> + Clone + Eq + PartialEq;

		//Information for the next session
		type NextSessionRotation: EstimateNextSessionRotation<Self::BlockNumber>;

		//one day block
		#[pallet::constant]
		type OneDay: Get<BlockNumberOf<Self>>;

		/// The pallet id
		type MyPalletId: Get<PalletId>;

		// randomness for seeds.
		type MyRandomness: Randomness<Option<Self::Hash>, Self::BlockNumber>;
	}

	///  bind user's redstone network address to other mail address (such ethereum address, moonbeam
	/// address, web2 address ...)
	#[pallet::storage]
	#[pallet::getter(fn contact_list)]
	pub type ContactList<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		MailAddress,
		BoundedVec<u8, ConstU32<128>>,
		OptionQuery,
	>;

	///
	#[pallet::storage]
	#[pallet::getter(fn mailing_list)]
	pub type MailingList<T: Config> = StorageNMap<
		_,
		(
			storage::Key<Twox64Concat, T::AccountId>,
			storage::Key<Blake2_128Concat, MailAddress>,
			storage::Key<Blake2_128Concat, u64>,
		),
		BoundedVec<u8, ConstU32<128>>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn map_triger)]
	pub(super) type MailMap<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<u8, ConstU32<128>>>;

	//Relevant time nodes for storage challenges
	#[pallet::storage]
	#[pallet::getter(fn challenge_duration)]
	pub(super) type ChallengeDuration<T: Config> = StorageValue<_, BlockNumberOf<T>, ValueQuery>;

	//Relevant time nodes for storage challenges
	#[pallet::storage]
	#[pallet::getter(fn verify_duration)]
	pub(super) type VerifyDuration<T: Config> = StorageValue<_, BlockNumberOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn cur_authority_index)]
	pub(super) type CurAuthorityIndex<T: Config> = StorageValue<_, u16, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn keys)]
	pub(super) type Keys<T: Config> =
		StorageValue<_, WeakBoundedVec<T::AuthorityId, T::StringLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn lock)]
	pub(super) type Lock<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		AddressBound(T::AccountId, BoundedVec<u8, ConstU32<128>>),

		SendMailSuccess(T::AccountId, Mail),
		UpdateAliasSuccess(T::AccountId, BoundedVec<u8, ConstU32<128>>),
		SetAliasSuccess(T::AccountId, BoundedVec<u8, ConstU32<128>>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		AddressBindDuplicate,
		/// Errors should have helpful documentation associated with them.
		MailSendDuplicate,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.

		#[pallet::weight(10_000)]
		pub fn bind_address(
			origin: OriginFor<T>,
			pmail_address: BoundedVec<u8, ConstU32<128>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(!MailMap::<T>::contains_key(&who), Error::<T>::AddressBindDuplicate);

			MailMap::<T>::insert(&who, pmail_address.clone());

			Self::deposit_event(Event::AddressBound(who.clone(), pmail_address.clone()));

			log::info!("-------bind address to pmail success: {:?}", pmail_address.clone());

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn set_alias(
			origin: OriginFor<T>,
			address: MailAddress,
			alias: BoundedVec<u8, ConstU32<128>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			match ContactList::<T>::get(&who, address.clone()) {
				Some(_) => {
					ContactList::<T>::mutate(&who, address.clone(), |v| *v = Some(alias.clone()));
					Self::deposit_event(Event::UpdateAliasSuccess(who.clone(), alias.clone()));
					log::info!("-------update alias success: {:?}", alias.clone());
				},
				None => {
					ContactList::<T>::insert(&who, address.clone(), alias.clone());
					Self::deposit_event(Event::SetAliasSuccess(who.clone(), alias.clone()));
					log::info!("-------add alias success: {:?}", alias.clone());
				},
			}

			Ok(())
		}

		/// send email
		#[pallet::weight(10_000)]
		pub fn send_mail(
			origin: OriginFor<T>,
			to: MailAddress,
			timestamp: u64,
			store_hash: BoundedVec<u8, ConstU32<128>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				!MailingList::<T>::contains_key((&who, to.clone(), timestamp)),
				Error::<T>::MailSendDuplicate
			);

			MailingList::<T>::insert((&who, to.clone(), timestamp), store_hash.clone());

			let mail = Mail { timestamp, store_hash };

			log::info!("------- mail send success: {:?}", mail);

			Self::deposit_event(Event::SendMailSuccess(who.clone(), mail));

			Ok(())
		}

		// #[pallet::weight(0)]
	}

	/// all notification will be send via offchain_worker, it is more efficient
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(now: T::BlockNumber) {
			log::info!("Hello world from mail-pallet workers!: {:?}", now);

			let deadline = Self::verify_duration();
			if sp_io::offchain::is_validator() {
				if now > deadline {
					//Determine whether to trigger a challenge
					if Self::trigger_challenge(now) {
						log::info!("offchain worker random challenge start");
						if let Err(e) = Self::offchain_work_start(now) {
							match e {
								OffchainErr::Working => log::info!(
									"offchain working, Unable to perform a new round of work."
								),
								_ => log::info!(
									"offchain worker generation challenge failed:{:?}",
									e
								),
							};
						}
						log::info!("offchain worker random challenge end");
					}
				}
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn offchain_work_start(now: BlockNumberOf<T>) -> Result<(), OffchainErr> {
			log::info!("get loacl authority...");
			let (authority_id, validators_index, validators_len) = Self::get_authority()?;
			log::info!("get loacl authority success!");
			if !Self::check_working(&now, &authority_id) {
				return Err(OffchainErr::Working)
			}
			log::info!("get challenge data...");
			// let challenge_map = Self::generation_challenge().map_err(|e| {
			// 	log::error!("generation challenge error:{:?}", e);
			// 	OffchainErr::GenerateInfoError
			// })?;
			log::info!("get challenge success!");
			log::info!("submit challenge to chain...");
			//Self::offchain_call_extrinsic(now, authority_id, challenge_map, validators_index,
			// validators_len)?;
			log::info!("submit challenge to chain!");
			Ok(())
		}

		fn check_working(now: &BlockNumberOf<T>, authority_id: &T::AuthorityId) -> bool {
			let key = &authority_id.encode();
			let storage = StorageValueRef::persistent(key);

			let res = storage.mutate(
				|status: Result<Option<BlockNumberOf<T>>, StorageRetrievalError>| {
					match status {
						// we are still waiting for inclusion.
						Ok(Some(last_block)) => {
							let lock_time = T::LockTime::get();
							if last_block + lock_time > *now {
								log::info!(
									"last_block: {:?}, lock_time: {:?}, now: {:?}",
									last_block,
									lock_time,
									now
								);
								Err(OffchainErr::Working)
							} else {
								Ok(*now)
							}
						},
						// attempt to set new status
						_ => Ok(*now),
					}
				},
			);

			if res.is_err() {
				log::error!("offchain work: {:?}", OffchainErr::Working);
				return false
			}

			true
		}

		//Trigger: whether to trigger the challenge
		fn trigger_challenge(now: BlockNumberOf<T>) -> bool {
			const START_FINAL_PERIOD: Permill = Permill::from_percent(80);

			let time_point = Self::random_time_number(20220509);
			//The chance to trigger a challenge is once a day
			let probability: u32 = T::OneDay::get().saturated_into();
			let range = LIMIT / probability as u64;
			if (time_point > 2190502) && (time_point < (range + 2190502)) {
				if let (Some(progress), _) =
					T::NextSessionRotation::estimate_current_session_progress(now)
				{
					if progress >= START_FINAL_PERIOD {
						log::error!("TooLate!");
						return false
					}
				}
				return true
			}
			false
		}

		// Generate a random number from a given seed.
		fn random_time_number(seed: u32) -> u64 {
			let (random_seed, _) = T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
			let random_seed = match random_seed {
				Some(v) => v,
				None => Default::default(),
			};
			let random_number = <u64>::decode(&mut random_seed.as_ref())
				.expect("secure hashes should always be bigger than u32; qed");
			random_number
		}

		fn get_authority() -> Result<(T::AuthorityId, u16, usize), OffchainErr> {
			let cur_index = <CurAuthorityIndex<T>>::get();
			let validators = Keys::<T>::get();
			//this round key to submit transationss
			let epicycle_key = match validators.get(cur_index as usize) {
				Some(id) => id,
				None => return Err(OffchainErr::UnexpectedError),
			};

			let mut local_keys = T::AuthorityId::all();

			if local_keys.len() == 0 {
				log::info!("no local_keys");
				return Err(OffchainErr::Ineligible)
			}

			local_keys.sort();

			let res = local_keys.binary_search(&epicycle_key);

			let authority_id = match res {
				Ok(index) => local_keys.get(index),
				Err(_e) => return Err(OffchainErr::Ineligible),
			};

			let authority_id = match authority_id {
				Some(id) => id,
				None => return Err(OffchainErr::Ineligible),
			};

			Ok((authority_id.clone(), cur_index, validators.len()))
		}
	}
}
