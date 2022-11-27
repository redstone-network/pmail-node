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
use frame_support::{traits::ConstU32, BoundedVec};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::cmp::{Eq, PartialEq};

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Mail {
	timestamp: u64,
	store_hash: BoundedVec<u8, ConstU32<128>>,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum MailAddress {
	SubAddr(BoundedVec<u8, ConstU32<32>>), // substrate address, start with 5...
	ETHAddr(BoundedVec<u8, ConstU32<32>>), // ethereum address, start with 0...
	MoonbeamAddr(BoundedVec<u8, ConstU32<32>>), // moonbeam address, start with 0...
	NormalAddr(BoundedVec<u8, ConstU32<32>>), // normal address, such as gmail, outlook.com...                                     //1@q.cn
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	///  bind user's redstone network address to other mail address (such ethereum address, moonbeam address, web2 address ...)
	#[pallet::storage]
	#[pallet::getter(fn contact_list)]
	pub type ContactList<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		MailAddress,
		BoundedVec<u8, ConstU32<256>>,
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
	pub(super) type MapMail<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<u8, ConstU32<128>>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		AddressBound(T::AccountId, MailAddress),

		SendMailSuccess(T::AccountId, MailAddress),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		AddressBindDuplicate,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
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
			mail_address: BoundedVec<u8, ConstU32<128>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// 校验是不是以pmail.io结尾

			log::info!("-------get bind mail address: {:?}", mail_address);

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn set_alias(
			origin: OriginFor<T>,
			address: MailAddress,
			alias: BoundedVec<u8, ConstU32<128>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			log::info!("-------get bind address: {:?}", address);
			log::info!("-------get bind alias: {:?}", alias);

			Ok(())
		}

		//  传入时间戳和哈嘻对用户来说是否足够友好
		#[pallet::weight(10_000)]
		pub fn send_mail(
			origin: OriginFor<T>,
			to:address: MailAddress,
			timestamp: u64,
			store_hash: BoundedVec<u8, ConstU32<128>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mail = Mail { timestamp, store_hash };

			log::info!("-------get bind alias: {:?}", mail);

			Ok(())
		}

		// #[pallet::weight(0)]
	}

	/// all notification will be send via offchain_worker, it is more efficient
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			log::info!("Hello world from mail-pallet workers!: {:?}", block_number);
		}
	}
}
