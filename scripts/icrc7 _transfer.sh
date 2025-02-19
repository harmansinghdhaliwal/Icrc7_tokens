dfx canister call icrc7 icrc7_transfer '(vec{
record{
to=record {
owner = principal "dfeex-vc3or-2abck-j24qg-vzytj-34nw4-cpb5r-h56yt-wx4ml-ly25s-yae";
subaccount = opt blob "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00";
};
token_id= 1;
from_subaccount= null;
memo= null;
created_at_time= null
}
})'


# dfx canister call icrc7 icrc7_transfer '(vec{
#     record{
#         to= record {
#             owner = principal "yatjp-hlagt-hqpjx-anqcv-iqntj-sv2ql-iar33-2e742-b6vmr-rirro-qqe";                                     
#             subaccount = opt blob "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00";
#         };
#         token_id= 2;
#         memo= opt blob "123";
#         from_subaccount= null;
#         created_at_time= null
#     }
# })'
