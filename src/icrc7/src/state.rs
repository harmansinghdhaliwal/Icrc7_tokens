use std::{cell::RefCell, collections::HashMap};

use crate::{
    errors::{ApproveTokenError, BurnError, InsertTransactionError, MintError, TransferError},
    icrc37_types::{
        ApprovalInfo, ApproveCollectionArg, ApproveCollectionResult, ApproveTokenArg,
        ApproveTokenResult, CollectionApprovalAccount, LedgerInfo, Metadata, TokenApprovalInfo,
    },
    icrc7_types::{
        BurnResult, Icrc7TokenMetadata, MintArg, MintResult, Transaction, TransactionType,
        TransferArg, TransferResult,
    },
    memory::{
        get_collection_approvals_memory, get_log_memory, get_token_approvals_memory,
        get_token_map_memory, Memory,
    },
    utils::{account_transformer, burn_account},
    BurnArg, SyncReceipt, TRANSACTION_TRANSFER_OP,
};
use candid::{CandidType, Decode, Encode, Principal};
use ic_stable_structures::{
    memory_manager::MemoryManager, storable::Bound, DefaultMemoryImpl, StableBTreeMap, Storable,
};
use icrc_ledger_types::{
    icrc::{generic_metadata_value::MetadataValue, generic_value::Value},
    icrc1::account::Account,
};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct Icrc7Token {
    pub token_id: u128,
    pub token_name: String,
    pub token_description: Option<String>,
    pub token_logo: Option<String>,
    pub token_owner: Account,
    pub approvals: Vec<ApprovalInfo>,
}

impl Storable for Icrc7Token {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).unwrap())
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl Icrc7Token {
    fn new(
        token_id: u128,
        token_name: String,
        token_description: Option<String>,
        token_logo: Option<String>,
        token_owner: Account,
    ) -> Self {
        Self {
            token_id,
            token_name,
            token_logo,
            token_owner,
            token_description,
            approvals: vec![],
        }
    }

    fn transfer(&mut self, to: Account) {
        self.token_owner = to;
        self.approvals.clear();
    }

    fn approval_check(&self, current_time: u64, account: &Account) -> bool {
        for approval in self.approvals.iter() {
            if approval.spender == *account {
                if approval.expires_at.is_none() {
                    return true;
                } else if approval.expires_at >= Some(current_time) {
                    return true;
                }
            }
        }
        false
    }

    fn token_metadata(&self) -> Icrc7TokenMetadata {
        let mut metadata = HashMap::<String, MetadataValue>::new();
        metadata.insert("Name".into(), MetadataValue::Text(self.token_name.clone()));
        metadata.insert(
            "Symbol".into(),
            MetadataValue::Text(self.token_name.clone()),
        );
        if let Some(ref description) = self.token_description {
            metadata.insert(
                "Description".into(),
                MetadataValue::Text(description.clone()),
            );
        }
        if let Some(ref logo) = self.token_logo {
            metadata.insert("logo".into(), MetadataValue::Text(logo.clone()));
        }
        metadata
    }

    fn burn(&mut self, burn_address: Account) {
        self.token_owner = burn_address;
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub minting_authority: Option<Account>,
    pub icrc7_symbol: String,
    pub icrc7_name: String,
    pub icrc7_description: Option<String>,
    pub icrc7_logo: Option<String>,
    pub icrc7_total_supply: u128,
    pub icrc7_supply_cap: Option<u128>,
    pub icrc7_max_query_batch_size: Option<u16>,
    pub icrc7_max_update_batch_size: Option<u16>,
    pub icrc7_max_take_value: Option<u16>,
    pub icrc7_default_take_value: Option<u16>,
    pub icrc7_max_memo_size: Option<u32>,
    pub icrc7_atomic_batch_transfers: Option<bool>,
    pub tx_window: Option<u64>,
    pub permitted_drift: Option<u64>,
    #[serde(skip, default = "get_token_map_memory")]
    pub tokens: StableBTreeMap<u128, Icrc7Token, Memory>,
    pub txn_count: u128,
    pub next_token_id: u128,
    #[serde(skip, default = "get_log_memory")]
    pub txn_log: StableBTreeMap<u128, Transaction, Memory>,
    pub archive_log_canister: Option<Principal>,
    pub sync_pending_txn_ids: Option<Vec<u128>>,
    pub archive_txn_count: u128,
    pub approval_ledger_info: LedgerInfo,
    #[serde(skip, default = "get_token_approvals_memory")]
    pub token_approvals: StableBTreeMap<u128, TokenApprovalInfo, Memory>,
    #[serde(skip, default = "get_collection_approvals_memory")]
    pub collection_approvals: StableBTreeMap<CollectionApprovalAccount, ApprovalInfo, Memory>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            minting_authority: None,
            icrc7_symbol: "ICRC7".into(),
            icrc7_name: "ICRC7 Collection".into(),
            icrc7_description: None,
            icrc7_logo: None,
            icrc7_total_supply: 0,
            icrc7_supply_cap: None,
            icrc7_max_query_batch_size: None,
            icrc7_max_update_batch_size: None,
            icrc7_max_take_value: None,
            icrc7_default_take_value: None,
            icrc7_max_memo_size: None,
            icrc7_atomic_batch_transfers: None,
            tx_window: None,
            permitted_drift: None,
            tokens: get_token_map_memory(),
            txn_count: 0,
            next_token_id: 0,
            txn_log: get_log_memory(),
            archive_log_canister: None,
            sync_pending_txn_ids: None,
            archive_txn_count: 0,
            approval_ledger_info: LedgerInfo::default(),
            token_approvals: get_token_approvals_memory(),
            collection_approvals: get_collection_approvals_memory(),
        }
    }
}

impl State {
    pub const DEFAULT_MAX_QUERY_BATCH_SIZE: u16 = 32;
    pub const DEFAULT_MAX_UPDATE_BATCH_SIZE: u16 = 32;
    pub const DEFAULT_TAKE_VALUE: u128 = 32;
    pub const DEFAULT_MAX_TAKE_VALUE: u128 = 32;
    pub const DEFAULT_MAX_MEMO_SIZE: u32 = 32;
    pub const DEFAULT_TX_WINDOW: u64 = 24 * 60 * 60 * 1000_000_000;
    pub const DEFAULT_PERMITTED_DRIFT: u64 = 2 * 60 * 1000_000_000;

    pub fn icrc7_symbol(&self) -> String {
        self.icrc7_symbol.clone()
    }

    pub fn icrc7_name(&self) -> String {
        self.icrc7_name.clone()
    }

    pub fn icrc7_description(&self) -> Option<String> {
        self.icrc7_description.clone()
    }

    pub fn icrc7_total_supply(&self) -> u128 {
        self.icrc7_total_supply
    }

    pub fn icrc7_supply_cap(&self) -> Option<u128> {
        self.icrc7_supply_cap
    }

    pub fn icrc7_logo(&self) -> Option<String> {
        self.icrc7_logo.clone()
    }

    pub fn icrc7_minting_authority(&self) -> Option<Account> {
        self.minting_authority.clone()
    }

    pub fn icrc7_max_query_batch_size(&self) -> Option<u16> {
        self.icrc7_max_query_batch_size
    }

    pub fn icrc7_max_update_batch_size(&self) -> Option<u16> {
        self.icrc7_max_update_batch_size
    }

    pub fn icrc7_default_take_value(&self) -> Option<u16> {
        self.icrc7_default_take_value
    }

    pub fn icrc7_max_take_value(&self) -> Option<u16> {
        self.icrc7_max_take_value
    }

    pub fn icrc7_max_memo_size(&self) -> Option<u32> {
        self.icrc7_max_memo_size
    }

    pub fn icrc7_atomic_batch_transfers(&self) -> Option<bool> {
        self.icrc7_atomic_batch_transfers
    }

    pub fn icrc7_owner_of(&self, token_id: &[u128]) -> Vec<Option<Account>> {
        let mut res = vec![None; token_id.len()];
        for (index, id) in token_id.iter().enumerate() {
            if let Some(ref token) = self.tokens.get(id) {
                res.insert(index, Some(token.token_owner))
            }
        }
        res
    }

    pub fn icrc37_metadata(&self) -> Metadata {
        let mut res = Metadata::new();
        if self
            .approval_ledger_info
            .max_approvals_per_token_or_collection
            > 0
        {
            res.insert(
                "icrc37:max_approvals_per_token_or_collection".to_string(),
                Value::Nat(
                    (self
                        .approval_ledger_info
                        .max_approvals_per_token_or_collection as u64)
                        .into(),
                ),
            );
        }
        if self.approval_ledger_info.max_revoke_approvals > 0 {
            res.insert(
                "icrc37:max_revoke_approvals".to_string(),
                Value::Nat((self.approval_ledger_info.max_revoke_approvals as u64).into()),
            );
        }
        res
    }

    pub fn get_archive_log_canister(&self) -> Option<Principal> {
        self.archive_log_canister
    }

    pub fn get_sync_pending_txn_ids(&self) -> Option<Vec<u128>> {
        self.sync_pending_txn_ids.clone()
    }

    pub fn set_sync_pending_txn_ids(&mut self, txn_ids: Option<Vec<u128>>) -> bool {
        self.sync_pending_txn_ids = txn_ids;
        return true;
    }

    fn txn_deduplication_check(
        &self,
        allowed_past_time: &u64,
        caller: &Account,
        args: &TransferArg,
    ) -> Result<(), TransferError> {
        let mut count = self.txn_count;
        while count != 0 {
            let txn = self.txn_log.get(&count).unwrap();
            if txn.ts < *allowed_past_time {
                return Ok(());
            }
            if txn.op == String::from(TRANSACTION_TRANSFER_OP) {
                if args.token_id == txn.tid
                    && caller == txn.from.as_ref().unwrap()
                    && args.to == txn.to.unwrap()
                    && args.memo == txn.memo
                    && args.created_at_time == Some(txn.ts)
                {
                    return Err(TransferError::Duplicate {
                        duplicate_of: count,
                    });
                } else {
                    count -= 1;
                    continue;
                }
            } else {
                count -= 1;
                continue;
            }
        }
        Ok(())
    }

    fn get_txn_id(&mut self) -> u128 {
        self.txn_count += 1;
        self.txn_count
    }

    fn log_transaction(
        &mut self,
        txn_type: TransactionType,
        at: u64,
        memo: Option<Vec<u8>>,
    ) -> u128 {
        let txn_id = self.get_txn_id();
        let txn = Transaction::new(txn_id, txn_type, at, memo);
        self.txn_log.insert(txn_id, txn);
        txn_id
    }

    fn get_current_txn_count(&self) -> u128 {
        self.txn_count - self.archive_txn_count
    }

    fn mock_transfer(
        &self,
        current_time: &u64,
        caller: &Account,
        arg: &TransferArg,
    ) -> Result<(), TransferError> {
        if let Some(time) = arg.created_at_time {
            let allowed_past_time = *current_time
                - self.tx_window.unwrap_or(State::DEFAULT_TX_WINDOW)
                - self
                    .permitted_drift
                    .unwrap_or(State::DEFAULT_PERMITTED_DRIFT);
            let allowed_future_time = *current_time
                + self
                    .permitted_drift
                    .unwrap_or(State::DEFAULT_PERMITTED_DRIFT);
            if time < allowed_past_time {
                return Err(TransferError::TooOld);
            } else if time > allowed_future_time {
                return Err(TransferError::CreatedInFuture {
                    ledger_time: current_time.clone(),
                });
            }
            self.txn_deduplication_check(&allowed_past_time, caller, arg)?;
        }
        // checking is token for the corresponding ID exists or not
        if let None = self.tokens.get(&arg.token_id) {
            return Err(TransferError::NonExistingTokenId);
        }
        if let Some(ref memo) = arg.memo {
            let max_memo_size = self
                .icrc7_max_memo_size
                .unwrap_or(State::DEFAULT_MAX_MEMO_SIZE);
            if memo.len() as u32 > max_memo_size {
                return Err(TransferError::GenericError {
                    error_code: 3,
                    message: "Exceeds Max Memo Size".into(),
                });
            }
        }
        // checking if receiver and sender have same address
        if arg.to == *caller {
            return Err(TransferError::InvalidRecipient);
        }
        let token = self.tokens.get(&arg.token_id).unwrap();
        // checking if the caller is authorized or is approve to make transaction
        if token.token_owner != *caller && !token.approval_check(current_time.clone(), caller) {
            return Err(TransferError::Unauthorized);
        }
        Ok(())
    }

    pub fn icrc7_transfer(
        &mut self,
        caller: &Principal,
        mut args: Vec<TransferArg>,
    ) -> Vec<Option<TransferResult>> {
        // checking if the argument length in 0
        if args.len() == 0 {
            return vec![Some(Err(TransferError::GenericBatchError {
                error_code: 1,
                message: "No Arguments Provided".into(),
            }))];
        }
        let max_update_batch_size = self
            .icrc7_max_query_batch_size
            .unwrap_or(State::DEFAULT_MAX_UPDATE_BATCH_SIZE);
        let mut txn_results = vec![None; args.len()];
        if args.len() as u16 > max_update_batch_size {
            txn_results[0] = Some(Err(TransferError::GenericBatchError {
                error_code: 2,
                message: "Exceed Max allowed Update Batch Size".into(),
            }));
            return txn_results;
        }
        if *caller == Principal::anonymous() {
            txn_results[0] = Some(Err(TransferError::GenericBatchError {
                error_code: 100,
                message: "Anonymous Identity".into(),
            }));
            return txn_results;
        }
        let current_time = ic_cdk::api::time();
        for (index, arg) in args.iter_mut().enumerate() {
            let caller_account = account_transformer(Account {
                owner: caller.clone(),
                subaccount: arg.from_subaccount,
            });
            arg.to = account_transformer(arg.to);
            if let Err(e) = self.mock_transfer(&current_time, &caller_account, &arg) {
                txn_results[index] = Some(Err(e));
            }
        }
        if let Some(true) = self.icrc7_atomic_batch_transfers {
            if txn_results
                .iter()
                .any(|res| res.is_some() && res.as_ref().unwrap().is_err())
            {
                return txn_results;
            }
        }
        for (index, arg) in args.iter().enumerate() {
            let caller_account = account_transformer(Account {
                owner: caller.clone(),
                subaccount: arg.from_subaccount,
            });
            let time = arg.created_at_time.unwrap_or(current_time);
            if let Some(Err(e)) = txn_results.get(index).unwrap() {
                match e {
                    TransferError::GenericBatchError {
                        error_code: _,
                        message: _,
                    } => return txn_results,
                    _ => continue,
                }
            }
            let mut token = self.tokens.get(&arg.token_id).unwrap();
            token.transfer(arg.to.clone());
            token.approvals.clear();
            self.tokens.insert(arg.token_id, token);
            let txn_id = self.log_transaction(
                TransactionType::Transfer {
                    tid: arg.token_id,
                    from: caller_account.clone(),
                    to: arg.to.clone(),
                },
                time,
                arg.memo.clone(),
            );
            txn_results[index] = Some(Ok(txn_id));
        }
        txn_results
    }

    fn mock_mint(&self, caller: &Account, arg: &MintArg) -> Result<(), MintError> {
        if let Some(cap) = self.icrc7_supply_cap {
            if cap == self.icrc7_total_supply {
                return Err(MintError::SupplyCapReached);
            }
        }
        if let None = self.minting_authority {
            return Err(MintError::GenericBatchError {
                error_code: 6,
                message: "Minting Authority Not Set".into(),
            });
        }
        if Some(*caller) != self.minting_authority {
            return Err(MintError::Unauthorized);
        }
        if let Some(ref memo) = arg.memo {
            let allowed_memo_length = self
                .icrc7_max_memo_size
                .unwrap_or(State::DEFAULT_MAX_MEMO_SIZE);
            if memo.len() as u32 > allowed_memo_length {
                return Err(MintError::GenericError {
                    error_code: 7,
                    message: "Exceeds Allowed Memo Length".into(),
                });
            }
        }
        if &arg.token_id < &self.next_token_id {
            return Err(MintError::TokenIdMinimumLimit);
        }
        if let Some(_) = self.tokens.get(&arg.token_id) {
            return Err(MintError::TokenIdAlreadyExist);
        }
        Ok(())
    }

    pub fn mint(&mut self, caller: &Principal, mut arg: MintArg) -> MintResult {
        let caller = account_transformer(Account {
            owner: caller.clone(),
            subaccount: arg.from_subaccount,
        });
        arg.to = account_transformer(arg.to);
        self.mock_mint(&caller, &arg)?;
        let token_name = arg.token_name.unwrap_or_else(|| {
            let name = format!("{} {}", self.icrc7_symbol, arg.token_id);
            name
        });
        let token = Icrc7Token::new(
            arg.token_id,
            token_name.clone(),
            arg.token_description.clone(),
            arg.token_logo,
            arg.to.clone(),
        );
        let token_metadata = token.token_metadata();
        self.tokens.insert(arg.token_id, token);
        self.next_token_id = arg.token_id + 1;

        let txn_id = self.log_transaction(
            TransactionType::Mint {
                tid: arg.token_id,
                from: caller,
                to: arg.to,
                meta: token_metadata,
            },
            ic_cdk::api::time(),
            arg.memo,
        );
        Ok(txn_id)
    }

    fn mock_burn(&self, caller: &Account, arg: &BurnArg) -> Result<(), BurnError> {
        if let Some(ref memo) = arg.memo {
            if memo.len() as u32
                > self
                    .icrc7_max_memo_size
                    .unwrap_or(State::DEFAULT_MAX_MEMO_SIZE)
            {
                return Err(BurnError::GenericError {
                    error_code: 3,
                    message: "Exceeds Max Memo Length".into(),
                });
            }
        }
        match self.tokens.get(&arg.token_id) {
            None => Err(BurnError::NonExistingTokenId),
            Some(ref token) => {
                if token.token_owner != *caller {
                    return Err(BurnError::Unauthorized);
                }
                Ok(())
            }
        }
    }

    pub fn burn(&mut self, caller: &Principal, mut args: Vec<BurnArg>) -> Vec<Option<BurnResult>> {
        if args.len() == 0 {
            return vec![Some(Err(BurnError::GenericBatchError {
                error_code: 1,
                message: "No Arguments Provided".into(),
            }))];
        }
        let mut txn_results = vec![None; args.len()];
        if *caller == Principal::anonymous() {
            txn_results[0] = Some(Err(BurnError::GenericBatchError {
                error_code: 100,
                message: "Anonymous Identity".into(),
            }));
            return txn_results;
        }
        for (index, arg) in args.iter_mut().enumerate() {
            let caller = account_transformer(Account {
                owner: caller.clone(),
                subaccount: arg.from_subaccount,
            });
            if let Err(e) = self.mock_burn(&caller, arg) {
                txn_results.insert(index, Some(Err(e)))
            }
        }
        if let Some(true) = self.icrc7_atomic_batch_transfers {
            if txn_results
                .iter()
                .any(|res| res.is_some() && res.as_ref().unwrap().is_err())
            {
                return txn_results;
            }
        }
        for (index, arg) in args.iter().enumerate() {
            let caller = account_transformer(Account {
                owner: caller.clone(),
                subaccount: arg.from_subaccount,
            });
            let burn_address = burn_account();
            if let Some(Err(e)) = txn_results.get(index).unwrap() {
                match e {
                    BurnError::GenericBatchError {
                        error_code: _,
                        message: _,
                    } => return txn_results,
                    _ => continue,
                }
            }
            let mut token = self.tokens.get(&arg.token_id).unwrap();
            token.burn(burn_address.clone());
            let tid = self.log_transaction(
                TransactionType::Burn {
                    tid: arg.token_id,
                    from: caller,
                    to: burn_address,
                },
                ic_cdk::api::time(),
                arg.memo.clone(),
            );
            txn_results.insert(index, Some(Ok(tid)))
        }
        txn_results
    }

    fn mock_approve(
        &self,
        caller: &Account,
        arg: &ApproveTokenArg,
    ) -> Result<(), ApproveTokenError> {
        if arg.approval_info.spender == *caller {
            return Err(ApproveTokenError::InvalidSpender);
        };
        if let Some(ref memo) = arg.approval_info.memo {
            let max_memo_size = self
                .icrc7_max_memo_size
                .unwrap_or(State::DEFAULT_MAX_MEMO_SIZE);
            if memo.len() as u32 > max_memo_size {
                return Err(ApproveTokenError::GenericError {
                    error_code: 3,
                    message: "Exceeds Max Memo Size".into(),
                });
            }
        };
        match self.tokens.get(&arg.token_id) {
            None => Err(ApproveTokenError::NonExistingTokenId),
            Some(ref token) => {
                if token.token_owner != *caller {
                    return Err(ApproveTokenError::NonExistingTokenId);
                }
                Ok(())
            }
        }
    }

    pub fn approve(
        &mut self,
        caller: &Principal,
        mut args: Vec<ApproveTokenArg>,
    ) -> Vec<Option<ApproveTokenResult>> {
        if args.len() == 0 {
            return vec![Some(Err(ApproveTokenError::GenericBatchError {
                error_code: 1,
                message: "No Arguments Provided".into(),
            }))];
        }

        let max_update_batch_size = self.icrc7_max_update_batch_size().unwrap_or_default();

        if args.len() > max_update_batch_size as usize {
            return vec![Some(Err(ApproveTokenError::GenericBatchError {
                error_code: 2,
                message: "Exceeds max update batch size".into(),
            }))];
        }

        let mut txn_results = vec![None; args.len()];

        for (index, arg) in args.iter_mut().enumerate() {
            let caller = account_transformer(Account {
                owner: caller.clone(),
                subaccount: arg.approval_info.from_subaccount,
            });
            if let Err(e) = self.mock_approve(&caller, arg) {
                txn_results.insert(index, Some(Err(e)))
            }
        }
        if let Some(true) = self.icrc7_atomic_batch_transfers {
            if txn_results
                .iter()
                .any(|res| res.is_some() && res.as_ref().unwrap().is_err())
            {
                return txn_results;
            }
        }

        for (index, arg) in args.iter().enumerate() {
            let caller = account_transformer(Account {
                owner: caller.clone(),
                subaccount: arg.approval_info.from_subaccount,
            });
            if let Some(Err(e)) = txn_results.get(index).unwrap() {
                match e {
                    &ApproveTokenError::GenericBatchError {
                        error_code: _,
                        message: _,
                    } => return txn_results,
                    _ => continue,
                }
            }

            match self.token_approvals.get(&arg.token_id) {
                None => {
                    let token_approval = TokenApprovalInfo::new(caller, arg.approval_info.clone());
                    self.token_approvals.insert(arg.token_id, token_approval);
                }
                Some(mut token_approval) => {
                    token_approval.approve(caller, arg.approval_info.clone());
                }
            }

            let tid = self.log_transaction(
                TransactionType::Approval {
                    tid: arg.token_id,
                    from: caller,
                    to: arg.approval_info.spender,
                    exp_sec: arg.approval_info.expires_at,
                },
                ic_cdk::api::time(),
                arg.approval_info.memo.clone(),
            );
            txn_results.insert(index, Some(Ok(tid)))
        }
        txn_results
    }

    pub fn collection_approve(
        &mut self,
        caller: &Principal,
        mut args: Vec<ApproveCollectionArg>,
    ) -> Vec<Option<ApproveCollectionResult>> {
        let txn_results: Vec<Option<ApproveCollectionResult>> = vec![None; args.len()];
        return txn_results;
    }

    pub fn icrc7_token_metadata(&self, token_ids: &[u128]) -> Vec<Option<Icrc7TokenMetadata>> {
        if token_ids.len() as u16
            > self
                .icrc7_max_query_batch_size
                .unwrap_or(State::DEFAULT_MAX_QUERY_BATCH_SIZE)
        {
            ic_cdk::trap("Exceeds Max Query Batch Size")
        }
        let mut metadata_list = vec![None; token_ids.len()];
        for (index, tid) in token_ids.iter().enumerate() {
            if let Some(ref token) = self.tokens.get(tid) {
                metadata_list.insert(index, Some(token.token_metadata()))
            }
        }
        metadata_list
    }

    pub fn icrc7_balance_of(&self, accounts: &[Account]) -> Vec<u128> {
        let mut count_list = vec![0; accounts.len()];
        accounts.iter().enumerate().for_each(|(index, account)| {
            self.tokens.iter().for_each(|(_id, ref token)| {
                if token.token_owner == *account {
                    let current_count = count_list[index];
                    count_list[index] = current_count + 1;
                }
            })
        });
        count_list
    }

    pub fn icrc7_tokens(&self, prev: Option<u128>, take: Option<u128>) -> Vec<u128> {
        let take = take.unwrap_or(State::DEFAULT_TAKE_VALUE);
        if take > State::DEFAULT_MAX_TAKE_VALUE {
            ic_cdk::trap("Exceeds Max Take Value")
        }
        let mut list: Vec<u128> = self.tokens.iter().map(|(k, _)| k).collect();
        list.sort();
        match prev {
            Some(prev) => match list.iter().position(|id| *id == prev) {
                None => vec![],
                Some(index) => list
                    .iter()
                    .map(|id| *id)
                    .skip(index)
                    .take(take as usize)
                    .collect(),
            },
            None => list[0..take as usize].to_vec(),
        }
    }

    pub fn icrc7_tokens_of(
        &self,
        account: Account,
        prev: Option<u128>,
        take: Option<u128>,
    ) -> Vec<u128> {
        let take = take.unwrap_or(State::DEFAULT_TAKE_VALUE);
        if take > State::DEFAULT_MAX_TAKE_VALUE {
            ic_cdk::trap("Exceeds Max Take Value")
        }
        let mut owned_tokens = vec![];
        for (id, token) in self.tokens.iter() {
            if token.token_owner == account {
                owned_tokens.push(id);
            }
        }
        owned_tokens.sort();
        match prev {
            None => owned_tokens[0..=take as usize].to_vec(),
            Some(prev) => match owned_tokens.iter().position(|id| *id == prev) {
                None => vec![],
                Some(index) => owned_tokens
                    .iter()
                    .map(|id| *id)
                    .skip(index)
                    .take(take as usize)
                    .collect(),
            },
        }
    }

    pub fn icrc7_txn_logs(&self, page_number: u32, page_size: u32) -> Vec<Transaction> {
        let offset = (page_number - 1) * page_size;
        if offset as u128 > self.get_current_txn_count() {
            ic_cdk::trap("Exceeds Max Offset Value")
        }
        let tx_logs = self
            .txn_log
            .iter()
            .skip(offset as usize)
            .take(page_size as usize)
            .map(|(_, txn)| txn.clone())
            .collect();

        tx_logs
    }

    pub fn get_txn_logs(&self, size: usize) -> Vec<Transaction> {
        let tx_logs: Vec<Transaction> = self
            .txn_log
            .iter()
            .take(size)
            .map(|(_, txn)| txn.clone())
            .collect();

        tx_logs
    }

    pub fn remove_txn_logs(&mut self, txn_ids: &Vec<u128>) -> bool {
        for txn_id in txn_ids {
            self.txn_log.remove(txn_id);
        }
        self.sync_pending_txn_ids = None;
        self.archive_txn_count += txn_ids.len() as u128;
        return true;
    }
}

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    pub static STATE: RefCell<State> = RefCell::default();
}

pub async fn call_sync_logs(
    archive_log_canister: Principal,
    txn_logs: Vec<Transaction>,
) -> SyncReceipt {
    // sync logs
    let call_result: Result<(SyncReceipt,), _> = ic_cdk::api::call::call(
        archive_log_canister,
        "insert_many_txn_log",
        (txn_logs.clone(),),
    )
    .await;

    match call_result {
        Ok(_) => Ok(txn_logs.len() as u32),
        Err((_rejection_code, _msg)) => Err(InsertTransactionError::RemoteError),
    }
}
