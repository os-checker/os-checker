use crate::prelude::*;

pub mod file_tree;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UserRepoPkg {
    pub user: XString,
    pub repo: XString,
    pub pkg: XString,
}

impl UserRepoPkg {
    pub fn to_repo(&self) -> UserRepo {
        let Self { user, repo, .. } = self;
        UserRepo {
            user: user.clone(),
            repo: repo.clone(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UserRepo {
    pub user: XString,
    pub repo: XString,
}
