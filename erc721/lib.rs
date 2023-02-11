// 標準ライブラリがなかったら標準ライブラリを使わない宣言
#![cfg_attr(not(feature = "std"), no_std)]

// Contract定義のエントリーポイント
#[ink::contract]
mod erc721 {
    use ink::prelude::string::{String, ToString};
    use ink::storage::Mapping; // inkからMapping structをimport.スマートコントラクト用に用意されているのでMapにはこれを使う。
    use scale::{Decode, Encode};

    pub type TokenId = u32; // TokenId

    // metadata.jsonのあるとこ
    const TOKEN_URI: &str = "https://example.com/";

    // ストレージ定義
    #[ink(storage)]
    #[derive(Default)] // Default traitを実装
    pub struct Erc721 {
        token_owner: Mapping<TokenId, AccountId>,
        token_approvals: Mapping<TokenId, AccountId>,
        owned_tokens_count: Mapping<AccountId, u32>,
        operator_approvals: Mapping<(AccountId, AccountId), ()>,
        token_id: TokenId,
    }

    // エラー定義
    #[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, Copy)] // いろいろtraitを実装
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

    // イベント定義

    // トークンがTransferされたときのイベント
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)] // indexedを追加
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: TokenId,
    }

    // 承認されたときのイベント
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        id: TokenId,
    }

    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    // コントラクトの実装
    impl Erc721 {
        // コンストラクタ
        #[ink(constructor)]
        pub fn new() -> Self {
            Erc721 {
                token_owner: Default::default(),
                token_approvals: Default::default(),
                owned_tokens_count: Default::default(),
                operator_approvals: Default::default(),
                token_id: 1, // 最初は１から
            }
        }

        // #[ink(message)]
        // 全てのパブリック関数はこの属性を使用する必要がある
        // 少なくとも一つの#[ink(message)]属性を持つ関数が定義されている必要がある
        // コントラクトと対話するための関数定義に使用

        // アカウントが持つトークンの数を返す
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            self.balance_of_or_zero(&owner)
        }

        #[ink(message)]
        pub fn token_uri(&self) -> String {
            String::from(TOKEN_URI) + &ToString::to_string(&self.token_id)
        }

        // トークンの所有者を取得する
        #[ink(message)]
        pub fn owner_of(&self, id: TokenId) -> Option<AccountId> {
            self.token_owner.get(id)
        }

        // 承認済みのアカウントIDを取得する
        #[ink(message)]
        pub fn get_approved(&self, id: TokenId) -> Option<AccountId> {
            self.token_approvals.get(id)
        }

        // 指定のアカウント間で全てApproveされているかどうか
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.approved_for_all(owner, operator)
        }

        // 指定のアカウントに対しての全承認をセットする
        #[ink(message)]
        pub fn set_approval_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
            self.approve_for_all(to, approved)?;
            Ok(())
        }

        // 指定のアカウントがトークンに対しての操作をApproveする
        #[ink(message)]
        pub fn approve(&mut self, to: AccountId, id: TokenId) -> Result<(), Error> {
            self.approve_for(&to, id)?;
            Ok(())
        }

        // トークンを移送
        #[ink(message)]
        pub fn transfer(&mut self, destinaion: AccountId, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            self.transfer_token_from(&caller, &destinaion, id)?;
            Ok(())
        }

        // トークンを指定のアカウントからアカウントへ移送
        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: TokenId,
        ) -> Result<(), Error> {
            self.transfer_token_from(&from, &to, id)?;
            Ok(())
        }

        // mint
        #[ink(message)]
        pub fn mint(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let id = self.token_id;
            self.add_token_to(&caller, id)?;

            // イベント発火
            self.env().emit_event(Transfer {
                from: Some(AccountId::from([0x0; 32])),
                to: Some(caller),
                id,
            });

            // インクリメント
            self.token_id += 1;

            Ok(())
        }

        // burn
        #[ink(message)]
        pub fn burn(&mut self, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let Self {
                token_owner,
                owned_tokens_count,
                ..
            } = self;

            let owner = token_owner.get(id).ok_or(Error::TokenNotFound)?;
            if owner != caller {
                return Err(Error::NotOwner);
            }

            // トークン所持情報削除
            let count = owned_tokens_count
                .get(caller)
                .map(|c| c - 1)
                .ok_or(Error::CannotFetchValue)?;
            owned_tokens_count.insert(caller, &count);
            token_owner.remove(id);

            // イベント発火
            self.env().emit_event(Transfer {
                from: Some(caller),
                to: Some(AccountId::from([0x0; 32])),
                id,
            });

            Ok(())
        }

        fn transfer_token_from(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            id: TokenId,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            if !self.exists(id) {
                return Err(Error::TokenNotFound);
            }

            if !self.approved_or_owner(Some(caller), id) {
                return Err(Error::NotApproved);
            }

            // Approval情報をクリア
            self.clear_approval(id);
            // トークンの所有情報を削除
            self.remove_token_from(from, id)?;
            // トークンの所有情報を追加
            self.add_token_to(to, id)?;

            // イベント発火
            self.env().emit_event(Transfer {
                from: Some(*from),
                to: Some(*to),
                id,
            });

            Ok(())
        }

        fn add_token_to(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
            let Self {
                token_owner,
                owned_tokens_count,
                ..
            } = self;

            // 既にトークン誰か持ってる
            if token_owner.contains(id) {
                return Err(Error::TokenExists);
            }

            // ゼロアドレス
            if *to == AccountId::from([0x0; 32]) {
                return Err(Error::NotAllowed);
            }

            let count = owned_tokens_count.get(to).map(|c| c + 1).unwrap_or(1);

            owned_tokens_count.insert(to, &count);
            token_owner.insert(id, to);

            Ok(())
        }

        fn clear_approval(&self, id: TokenId) {
            self.token_approvals.remove(id);
        }

        fn remove_token_from(&mut self, from: &AccountId, id: TokenId) -> Result<(), Error> {
            // 構造体からフィールドを取り出す
            let Self {
                token_owner,
                owned_tokens_count,
                ..
            } = self;

            // トークンがない
            if !token_owner.contains(id) {
                return Err(Error::TokenNotFound);
            }

            let count = owned_tokens_count
                .get(from) // トークンの所有数
                .map(|c| c - 1) // 1減らす
                .ok_or(Error::CannotFetchValue)?; // 見つからなかったらエラー返す

            // トークン所有数を更新
            owned_tokens_count.insert(from, &count);
            // トークン所有者を削除する
            token_owner.remove(id);

            Ok(())
        }

        // 指定のアドレスが所有者　または　指定のトークンに対してのApprovalがある　または　allでApprovalされてる
        fn approved_or_owner(&self, from: Option<AccountId>, id: TokenId) -> bool {
            let owner = self.owner_of(id);
            from != Some(AccountId::from([0x0; 32]))
                && (from == owner
                    || from == self.token_approvals.get(id)
                    || self.approved_for_all(
                        owner.expect("Error with AccountId"),
                        from.expect("Error with AccountId"),
                    ))
        }

        fn exists(&self, id: TokenId) -> bool {
            self.token_owner.contains(id)
        }

        fn approve_for(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
            // 呼び出しもと
            let caller = self.env().caller();
            // トークン所有者
            let owner = self.owner_of(id);

            // 呼び出しもとと所有者が同じまたは、既にApproveされてる
            if !(owner == Some(caller)
                || self.approved_for_all(owner.expect("Error with AccountId"), caller))
            {
                return Err(Error::NotAllowed);
            }

            // 0アドレス
            if *to == AccountId::from([0x0; 32]) {
                return Err(Error::NotAllowed);
            }

            // ストレージに追加
            if self.token_approvals.contains(id) {
                return Err(Error::CannotInsert);
            } else {
                self.token_approvals.insert(id, to);
            }

            // イベント発火
            self.env().emit_event(Approval {
                from: caller,
                to: *to,
                id,
            });

            Ok(())
        }

        fn approve_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if to == caller {
                return Err(Error::NotAllowed);
            }

            // イベント発火
            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator: to,
                approved,
            });

            if approved {
                self.operator_approvals.insert((&caller, &to), &());
            } else {
                self.operator_approvals.remove((&caller, &to));
            }

            Ok(())
        }

        fn balance_of_or_zero(&self, of: &AccountId) -> u32 {
            self.owned_tokens_count.get(of).unwrap_or(0)
        }

        fn approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operator_approvals.contains((&owner, &operator))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn mint_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut erc721 = Erc721::new();

            // まだトークンがmintされていないので所有者はいない
            assert_eq!(erc721.owner_of(1), None);
            // デフォルトユーザーでまだmintしていないのでトークンをもっていない
            assert_eq!(erc721.balance_of(accounts.alice), 0);
            // mint成功するはず
            assert_eq!(erc721.mint(), Ok(()));
            // mintしたのでトークンを所有しているはず
            assert_eq!(erc721.balance_of(accounts.alice), 1);
        }
    }
}
