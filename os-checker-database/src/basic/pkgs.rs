use std::{cmp::Ordering, collections::HashMap};

use os_checker_types::Cmd;
use serde::{Deserialize, Serialize};

use crate::utils::new_map_with_cap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pkgs {
    inner: Vec<Pkg>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Pkg {
    pub pkg: String,
    pub count: usize,
}

// *******************

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Aggregate<T> {
    inner: Vec<T>,
}

impl<T: BasicItem> Aggregate<T> {
    pub fn with_capacity(cap: usize) -> Self {
        let mut inner = Vec::<T>::with_capacity(cap + 1);
        // index 0 always refers to all
        inner.push(T::all_with_0count());
        Aggregate { inner }
    }

    pub fn push(&mut self, item: &str, cmds: &[&Cmd]) {
        let ele = T::new_from_cmd(item, cmds);
        self.inner[0].increment_all(ele.count());
        self.inner.push(ele);
    }

    pub fn from_map(map: HashMap<&str, Vec<&Cmd>>) -> Self {
        let mut targets = Self::with_capacity(map.len());
        for (triple, cmds) in map {
            targets.push(triple, &cmds);
        }
        // 降序排列
        targets.inner.sort_unstable_by(T::sort_descending);
        targets
    }

    pub fn merge_batch(v: Vec<Self>) -> Self {
        let mut map = new_map_with_cap::<String, usize>(24);
        for targets in v {
            for ele in targets.inner {
                let (name, count) = ele.split();
                map.entry(name).and_modify(|c| *c += count).or_insert(count);
            }
        }
        // 降序排列
        map.sort_unstable_by(|k1, &v1, k2, &v2| (v2, &**k2).cmp(&(v1, &**k1)));

        let inner = map
            .into_iter()
            .map(|(name, count)| T::new(name, count))
            .collect();
        Self { inner }
    }
}

pub trait BasicItem: Sized {
    /// name of All-*
    const ALL: &str;

    /// default zero count on All-
    fn all_with_0count() -> Self {
        Self::new(Self::ALL.to_owned(), 0)
    }

    /// increment All- by 1
    fn increment_all(&mut self, new_count: usize) {
        *self.count_mut() += new_count;
    }

    /// normal item construction
    fn new_from_cmd(item: &str, cmds: &[&Cmd]) -> Self {
        Self::new(item.to_owned(), cmds.iter().map(|c| c.count).sum())
    }

    fn name(&self) -> &str;
    fn count(&self) -> usize;
    fn count_mut(&mut self) -> &mut usize;

    fn sort_descending(&self, other: &Self) -> Ordering {
        (other.count(), self.name()).cmp(&(self.count(), other.name()))
    }

    /// split into name and count
    fn split(self) -> (String, usize);
    /// construct from name and count
    fn new(name: String, count: usize) -> Self;
}

pub type Targets = Aggregate<Target>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Target {
    triple: String,
    count: usize,
}

impl BasicItem for Target {
    const ALL: &str = "All-Targets";

    fn name(&self) -> &str {
        &self.triple
    }

    fn count(&self) -> usize {
        self.count
    }

    fn count_mut(&mut self) -> &mut usize {
        &mut self.count
    }

    fn split(self) -> (String, usize) {
        (self.triple, self.count)
    }

    fn new(triple: String, count: usize) -> Self {
        Self { triple, count }
    }
}

//
// impl Target {
//     fn all_targets_with_0count() -> Self {
//         Target {
//             triple: "All-Targets".to_owned(),
//             count: 0,
//         }
//     }
//
//     fn new(triple: &str, cmds: &[&Cmd]) -> Self {
//         Target {
//             triple: triple.to_owned(),
//             count: cmds.iter().map(|c| c.count).sum(),
//         }
//     }
// }
