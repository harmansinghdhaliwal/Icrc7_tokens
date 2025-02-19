//use ic_cdk_macros::export_candid;

use crate::cycles::WalletReceiveResult;
use crate::icrc37_types::*;
use crate::icrc3_types::*;
use crate::icrc7_types::*;
//use candid::export_service;
use candid::{Nat, Principal};
//use ic_cdk_macros::query;
use icrc_ledger_types::{icrc1::account::Account, icrc3::blocks::DataCertificate};

pub mod icrc7_types;
pub mod icrc37_types;
pub mod icrc3_types;
pub mod init_method;
pub mod memory;
pub mod query_method;
pub mod icrc37_query_method;
pub mod icrc3_query_method;
pub mod state;
pub mod update_method;
pub mod icrc37_update_method;
pub mod cycles;
pub mod utils;
pub mod candid_file_generator;
pub mod guards;
pub mod errors;
pub mod archive;

//use icrc7_types::*;


//export_candid!();
ic_cdk::export_candid!();
