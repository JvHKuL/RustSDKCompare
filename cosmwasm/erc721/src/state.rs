use cosmwasm_std::Addr;
use cw_storage_plus::Map;

/// A token ID.
pub type TokenId = u32;

pub struct Erc721<'a> {
    /// Mapping from token to owner.
    pub token_owner: Map<'a, TokenId, String>,
    /// Mapping from token to approvals users.
    pub token_approvals: Map<'a, TokenId, String>,
    /// Mapping from owner to number of owned token.
    pub owned_tokens_count: Map<'a, &'a Addr, u32>,
}

impl Default for Erc721<'static> {
    fn default() -> Self {
        Self::new("token_owner", "token_approvals", "owned_tokens_count_key")
    }
}

impl<'a> Erc721<'a> {
    fn new(
        token_owner_key: &'a str,
        token_approvals_key: &'a str,
        owned_tokens_count_key: &'a str,
    ) -> Self {
        Self {
            token_owner: Map::new(token_owner_key),
            token_approvals: Map::new(token_approvals_key),
            owned_tokens_count: Map::new(owned_tokens_count_key),
        }
    }
}
