use super::*;
use crate::{mock::*, Error};

use frame_support::{assert_noop, assert_ok};

use mock::RuntimeEvent as Event;
use sp_core::sr25519::Public;

#[test]
fn it_works_for_bind_address() {
	new_test_ext().execute_with(|| {
		let signer = Public::from_raw([0; 32]);
		let pmail_address: BoundedVec<u8, ConstU32<128>> = vec![1, 2, 3].try_into().unwrap();

		// assert
		assert_ok!(MailModule::bind_address(RuntimeOrigin::signed(signer), pmail_address.clone()));

		// assert transfer limit owner
		assert_eq!(MailMap::<Test>::get(signer), Some(pmail_address.clone()));
		assert_eq!(OwnerMap::<Test>::get(pmail_address.clone()), Some(signer));

		// assert successful events
		System::assert_has_event(Event::MailModule(crate::Event::AddressBound(
			signer,
			pmail_address,
		)));
	});
}

#[test]
fn it_works_for_set_alias() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		// assert_ok!(Mail::bind_address(RuntimeOrigin::signed(1), 42));
		// Read pallet storage and assert an expected result.
	});
}

#[test]
fn it_works_for_send_mail() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		// assert_ok!(Mail::bind_address(RuntimeOrigin::signed(1), 42));
		// Read pallet storage and assert an expected result.
	});
}

#[test]
fn it_will_fail_when_bind_address_duplicate() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		// assert_noop!(Mail::bind_address(RuntimeOrigin::signed(1)), Error::<Test>::NoneValue);
	});
}
