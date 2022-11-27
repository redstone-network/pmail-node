use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works_for_bind_address() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		// assert_ok!(Mail::bind_address(RuntimeOrigin::signed(1), 42));
		// Read pallet storage and assert an expected result.
	});
}

#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		// assert_noop!(Mail::bind_address(RuntimeOrigin::signed(1)), Error::<Test>::NoneValue);
	});
}
