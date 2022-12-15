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

pub const MAIL_SUFFIX: &str = "@pmailbox.org";
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

enum OffchainErr {
	Working,
}

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::Working =>
				write!(fmt, "The offline working machine is currently executing work"),
		}
	}
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
	use serde::{Deserialize, Deserializer};
	use sp_runtime::{Permill, SaturatedConversion};
	use sp_std::{borrow::ToOwned, vec::Vec};

	pub const LIMIT: u64 = u64::MAX;

	/*
	{
	"code": 0,
	"data": [
	  {
			"subject": "test",
			"body": "<meta http-equiv=\"Content-Type\" content=\"text/html; charset=GB18030\"><div>hello, boy. how are you.</div><div><br></div><div><div style=\"color:#909090;font-family:Arial Narrow;font-size:12px\">------------------</div><div style=\"font-size:14px;font-family:Verdana;color:#000;\"><a class=\"xm_write_card\" id=\"in_alias\" style=\"white-space: normal; display: inline-block; text-decoration: none !important;font-family: -apple-system,BlinkMacSystemFont,PingFang SC,Microsoft YaHei;\" href=\"https://wx.mail.qq.com/home/index?t=readmail_businesscard_midpage&amp;nocheck=true&amp;name=%E5%B0%8F%E7%99%BD%E9%BE%99&amp;icon=http%3A%2F%2Fthirdqq.qlogo.cn%2Fg%3Fb%3Dsdk%26k%3Diby9h7f0AjE5pUic9pIt3ynw%26s%3D100%26t%3D1556660321%3Frand%3D1650372662&amp;mail=116174160%40qq.com&amp;code=\" target=\"_blank\"><table style=\"white-space: normal;table-layout: fixed; padding-right: 20px;\" contenteditable=\"false\" cellpadding=\"0\" cellspacing=\"0\"><tbody><tr valign=\"top\"><td style=\"width: 40px;min-width: 40px; padding-top:10px\"><div style=\"width: 38px; height: 38px; border: 1px #FFF solid; border-radius:50%; margin: 0;vertical-align: top;box-shadow: 0 0 10px 0 rgba(127,152,178,0.14);\"><img src=\"http://thirdqq.qlogo.cn/g?b=sdk&amp;k=iby9h7f0AjE5pUic9pIt3ynw&amp;s=100&amp;t=1556660321?rand=1650372662\" style=\"width:100%;height:100%;border-radius:50%;pointer-events: none;\"></div></td><td style=\"padding: 10px 0 8px 10px;\"><div class=\"businessCard_name\" style=\"font-size: 14px;color: #33312E;line-height: 20px; padding-bottom: 2px; margin:0;font-weight: 500;\">小白龙</div><div class=\"businessCard_mail\" style=\"font-size: 12px;color: #999896;line-height: 18px; margin:0;\">116174160@qq.com</div></td></tr></tbody></table></a></div></div><div>&nbsp;</div>",
			"from": [{
				"Name": "=?gb18030?B?0KGw18H6?=",
				"Address": "116174160@qq.com"
			}],
			"to": [{
				"Name": "=?gb18030?B?dGVzdDE=?=",
				"Address": "test1@pmailbox.org"
			}],
			"data": "2022-12-04T17:52:21+08:00"
		}
		]
	}
	*/

	#[derive(Deserialize, Encode, Decode, Default, RuntimeDebug)]
	struct AddressInfo {
		#[serde(deserialize_with = "de_string_to_bytes")]
		name: Vec<u8>,
		#[serde(deserialize_with = "de_string_to_bytes")]
		address: Vec<u8>,
	}

	#[derive(Deserialize, Encode, Decode, Default, RuntimeDebug)]
	struct MailInfo {
		#[serde(deserialize_with = "de_string_to_bytes")]
		subject: Vec<u8>,
		#[serde(deserialize_with = "de_string_to_bytes")]
		body: Vec<u8>,

		from: Vec<AddressInfo>,
		to: Vec<AddressInfo>,

		#[serde(deserialize_with = "de_string_to_bytes")]
		data: Vec<u8>,

		timestampe: u64,
	}

	#[derive(Deserialize, Encode, Decode, Default, RuntimeDebug)]
	struct MailListResponse {
		#[serde(deserialize_with = "de_string_to_bytes")]
		data: Vec<u8>,
		code: u64,
		#[serde(deserialize_with = "de_string_to_bytes")]
		msg: Vec<u8>,
	}

	pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s: &str = Deserialize::deserialize(de)?;
		Ok(s.as_bytes().to_vec())
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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
	#[pallet::getter(fn map_mail)]
	pub(super) type MailMap<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<u8, ConstU32<128>>>;

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
			if sp_io::offchain::is_validator() {
				Self::offchain_work_start(now);
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn offchain_work_start(now: T::BlockNumber) -> Result<(), OffchainErr> {
			for (account_id, username) in MailMap::<T>::iter() {}

			Ok(())
		}
	}

	fn get_email_from_web2(username: &str) -> Result<u64, http::Error> {
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(10_000));

		let url =
			"http://127.0.0.1:8888/api/mails/list?emailname=".to_owned() + username + MAIL_SUFFIX;

		// let url = "http://mail1.pmailbox.org:8888/api/mails/list?emailname=".to_owned() +
		// 	username + MAIL_SUFFIX;

		let request = http::Request::get(&url).add_header("content-type", "application/json");

		let pending = request.deadline(deadline).send().map_err(|e| {
			log::info!("####post pending error: {:?}", e);
			http::Error::IoError
		})?;

		let response = pending.try_wait(deadline).map_err(|e| {
			log::info!("####post response error: {:?}", e);
			http::Error::DeadlineReached
		})??;

		if response.code != 200 {
			log::info!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}

		let body = response.body().collect::<Vec<u8>>();

		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::info!("No UTF8 body");
			http::Error::Unknown
		})?;

		if "ok" != body_str {
			log::info!("publish task fail: {}", body_str);
			return Err(http::Error::Unknown)
		}

		Ok(0)
	}
}
