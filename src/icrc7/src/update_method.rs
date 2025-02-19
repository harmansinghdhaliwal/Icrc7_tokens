use candid::Principal;
use ic_cdk::update;

use crate::{
    guards::owner_guard, state::STATE, BurnArg, BurnResult, MintArg, MintBatchArgs, MintBatchCustomerArgs, MintBatchResult, MintResult, TransferArg, TransferResult
};
use icrc_ledger_types::icrc1::account::Account;

#[update]
pub fn icrc7_transfer(args: Vec<TransferArg>) -> Vec<Option<TransferResult>> {
    let caller = ic_cdk::caller();
    STATE.with(|s| s.borrow_mut().icrc7_transfer(&caller, args))
}

#[update]
pub fn mint(arg: MintArg) -> MintResult {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        return Err(crate::errors::MintError::GenericBatchError {
            error_code: 100,
            message: "Anonymous Identity".into(),
        });
    }
    STATE.with(|s| s.borrow_mut().mint(&caller, arg, None))
}

#[update]
pub fn mint_batch_supplier(args: MintBatchArgs) -> MintBatchResult {
    let caller = ic_cdk::caller();

    if caller == Principal::anonymous() {
        return Err(crate::errors::MintError::GenericBatchError {
            error_code: 100,
            message: "Anonymous Identity".into(),
        });
    }

    STATE.with(|s| s.borrow_mut().mint_batch_supplier(&caller, args))
}

#[update]
pub fn mint_batch_customer(args: MintBatchCustomerArgs) -> MintBatchResult {
    let caller = ic_cdk::caller();

    if caller == Principal::anonymous() {
        return Err(crate::errors::MintError::GenericBatchError {
            error_code: 100,
            message: "Anonymous Identity".into(),
        });
    }
    STATE.with(|s| s.borrow_mut().mint_batch_customer(&caller, args))
}


#[update]
pub fn burn(args: Vec<BurnArg>) -> Vec<Option<BurnResult>> {
    let caller = ic_cdk::caller();
    STATE.with(|s| s.borrow_mut().burn(&caller, args))
}

#[update(guard = "owner_guard")]
pub fn set_minting_authority(minting_account: Account) -> bool {
    STATE.with(|s| s.borrow_mut().minting_authority = Some(minting_account));
    return true;
}
