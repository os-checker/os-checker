use crate::utils::{group_by, new_map_with_cap};
use os_checker_types::{Cmd, JsonOutput};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cmp::Ordering};

// ******************* Pkgs & Pkg *******************

pub type Pkgs = Aggregate<Pkg>;

impl Pkgs {
    pub fn new<'a, I>(cmds: I, json: &'a JsonOutput) -> Self
    where
        I: IntoIterator<Item = &'a Cmd>,
    {
        Self::from_map(cmds, |cmd| {
            let pkg_idx = dbg!(cmd.package_idx);
            dbg!(&json.env.packages[pkg_idx].name)
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pkg {
    pkg: String,
    count: usize,
}

impl BasicItem for Pkg {
    const ALL: &str = "All-Pkgs";

    fn name(&self) -> &str {
        &self.pkg
    }

    fn count(&self) -> usize {
        self.count
    }

    fn count_mut(&mut self) -> &mut usize {
        &mut self.count
    }

    fn split(self) -> (String, usize) {
        (self.pkg, self.count)
    }

    fn new(pkg: String, count: usize) -> Self {
        Self { pkg, count }
    }
}

// ******************* Checkers & Checker *******************

pub type Checkers = Aggregate<Checker>;

impl Checkers {
    pub fn new<'a, I>(cmds: I) -> Self
    where
        I: IntoIterator<Item = &'a Cmd>,
    {
        Self::from_map(cmds, |cmd| cmd.tool.as_str())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Checker {
    checker: String,
    count: usize,
}

impl BasicItem for Checker {
    const ALL: &str = "All-Checkers";

    fn name(&self) -> &str {
        &self.checker
    }

    fn count(&self) -> usize {
        self.count
    }

    fn count_mut(&mut self) -> &mut usize {
        &mut self.count
    }

    fn split(self) -> (String, usize) {
        (self.checker, self.count)
    }

    fn new(checker: String, count: usize) -> Self {
        Self { checker, count }
    }
}

// ******************* Targets & Target *******************

pub type Targets = Aggregate<Target>;

impl Targets {
    pub fn new<'a, I>(cmds: I) -> Self
    where
        I: IntoIterator<Item = &'a Cmd>,
    {
        Self::from_map(cmds, |cmd| &*cmd.target_triple)
    }
}

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

// ******************* FeaturesSets & Features *******************

pub type FeaturesSets = Aggregate<Features>;

impl FeaturesSets {
    pub fn new<'a, I>(cmds: I) -> Self
    where
        I: IntoIterator<Item = &'a Cmd>,
    {
        Self::from_map(cmds, |cmd| cmd.features.join(" "))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Features {
    features: String,
    count: usize,
}

impl BasicItem for Features {
    const ALL: &str = "All-Features-Sets";

    fn name(&self) -> &str {
        &self.features
    }

    fn count(&self) -> usize {
        self.count
    }

    fn count_mut(&mut self) -> &mut usize {
        &mut self.count
    }

    fn split(self) -> (String, usize) {
        (self.features, self.count)
    }

    fn new(features: String, count: usize) -> Self {
        Self { features, count }
    }
}

// ******************* Aggregate<T> *******************

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

    pub fn push(&mut self, item: String, cmds: &[&Cmd]) {
        let ele = T::new_from_cmd(item, cmds);
        self.inner[0].increment_all(ele.count());
        self.inner.push(ele);
    }

    fn from_map<'a, I, K>(cmds: I, mut f: impl FnMut(&&'a Cmd) -> K) -> Self
    where
        I: IntoIterator<Item = &'a Cmd>,
        K: Into<Cow<'a, str>>,
    {
        let map = group_by(cmds, |cmd| f(cmd).into());
        let mut targets = Self::with_capacity(map.len());
        for (triple, cmds) in map {
            targets.push(triple.into_owned(), &cmds);
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

// ******************* BasicItem for T in Aggregate<T> *******************

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
    fn new_from_cmd(item: String, cmds: &[&Cmd]) -> Self {
        Self::new(item, cmds.iter().map(|c| c.count).sum())
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
