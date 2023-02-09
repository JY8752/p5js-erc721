// 標準ライブラリがなかったら標準ライブラリを使わない宣言
#![cfg_attr(not(feature = "std"), no_std)]

// Contract定義のエントリーポイント
#[ink::contract]
mod erc721 {
    use ink::storage::Mapping; // inkからMapping structをimport.スマートコントラクト用に用意されているのでMapにはこれを使う。
    use scale::{Decode, Encode}; //

    pub type TokenId = u32; // TokenId

    // ストレージ定義
    #[ink(storage)]
    #[derive(Default)] // Default traitを実装
    pub struct Erc721 {
        token_owner: Mapping<TokenId, AccountId>,
        token_approvals: Mapping<TokenId, AccountId>,
        owned_tokens_count: Mapping<AccountId, u32>,
        operator_approvals: Mapping<(AccountId, AccountId), ()>,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, Copy)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotOwner,
        NotApproved,
        TokenExists,
        TokenNotFound,
        CannotInsert,
        CannotFetchValue,
        NotAllowed,
    }

    impl Erc721 {
        #[ink(constructor)]
        pub fn new() -> Self {
            Default::default()
        }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            // self.balance_of_or_zero(&owner)
            1
        }
    }

    // #[cfg(test)]
    // mod tests {
    //     /// Imports all the definitions from the outer scope so we can use them here.
    //     use super::*;

    //     /// We test if the default constructor does its job.
    //     #[ink::test]
    //     fn default_works() {
    //         let erc721 = Erc721::default();
    //         assert_eq!(erc721.get(), false);
    //     }

    //     /// We test a simple use case of our contract.
    //     #[ink::test]
    //     fn it_works() {
    //         let mut erc721 = Erc721::new(false);
    //         assert_eq!(erc721.get(), false);
    //         erc721.flip();
    //         assert_eq!(erc721.get(), true);
    //     }
    // }
}
