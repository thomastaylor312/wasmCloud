use std::{
    collections::{hash_map::Entry, BTreeSet, HashMap, HashSet},
    hash::Hash,
    sync::Arc,
};

use wasmcloud_control_interface::{InterfaceLinkDefinition, WitInterface};

#[derive(Debug, Eq)]
pub struct LinkKey {
    link: Arc<InterfaceLinkDefinition>,
}

impl PartialEq for LinkKey {
    fn eq(&self, other: &Self) -> bool {
        self.link.source_id == other.link.source_id
            && self.link.name == other.link.name
            && self.link.wit_namespace == other.link.wit_namespace
            && self.link.wit_package == other.link.wit_package
    }
}

impl Hash for LinkKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.link.source_id.hash(state);
        self.link.name.hash(state);
        self.link.wit_namespace.hash(state);
        self.link.wit_package.hash(state);
    }
}

impl From<Arc<InterfaceLinkDefinition>> for LinkKey {
    fn from(link: Arc<InterfaceLinkDefinition>) -> Self {
        Self { link }
    }
}

impl From<InterfaceLinkDefinition> for LinkKey {
    fn from(link: InterfaceLinkDefinition) -> Self {
        Self {
            link: Arc::new(link),
        }
    }
}

impl LinkKey {
    /// Creates a LinkKey from the given source_id, name, wit_namespace, and wit_package. Generally for use when deleting
    pub fn new(source_id: &str, name: &str, wit_namespace: &str, wit_package: &str) -> Self {
        let link = Arc::new(InterfaceLinkDefinition {
            source_id: source_id.to_string(),
            name: name.to_string(),
            wit_namespace: wit_namespace.to_string(),
            wit_package: wit_package.to_string(),
            ..Default::default()
        });
        Self { link }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Value {
    link: Arc<InterfaceLinkDefinition>,
    interfaces: BTreeSet<WitInterface>,
}

impl From<Arc<InterfaceLinkDefinition>> for Value {
    fn from(link: Arc<InterfaceLinkDefinition>) -> Self {
        let interfaces = link.interfaces.clone().into_iter().collect();
        Self { link, interfaces }
    }
}

/// A struct for holding a collection of [`InterfaceLinkDefinition`]s and their associated target
/// interfaces. This structure ensures that all interfaces for each package are disjoint when
/// inserting
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Links {
    /// This maps a partial set of link data to a set of link definitions that have the same core
    /// name, source, and wit package identifier, but different interfaces
    links: HashMap<LinkKey, HashSet<Value>>,
}

impl Links {
    pub fn new() -> Self {
        Self::default()
    }

    // Do we need a get? I didn't see a get anywhere in our code, only iteration

    /// Insert the given link into the collection of links. If the link has non-disjoint interfaces,
    /// this function will return an error
    pub fn insert(&mut self, link: InterfaceLinkDefinition) -> anyhow::Result<()> {
        let link = Arc::new(link);
        let val = Value::from(link.clone());
        match self.links.entry(LinkKey { link }) {
            Entry::Occupied(mut entry) => {
                let current = entry.get_mut();
                if current
                    .iter()
                    .map(|val| &val.interfaces)
                    .any(|i| i.intersection(&val.interfaces).count() > 0)
                {
                    // Should we include the interfaces here?
                    return Err(anyhow::anyhow!("Links between the same component and package must have disjoint (non overlapping) interfaces"));
                }
                current.insert(val);
            }
            Entry::Vacant(entry) => {
                entry.insert(HashSet::from([val]));
            }
        }
        Ok(())
    }

    /// Removes all links for the given key. Returns true if the key was found and removed.
    pub fn remove(&mut self, key: impl Into<LinkKey>) -> bool {
        self.links.remove(&key.into()).is_some()
    }

    /// Returns an iterator over all of the links in this collection
    pub fn iter(&self) -> impl Iterator<Item = &InterfaceLinkDefinition> {
        self.links
            .values()
            .flat_map(|vals| vals.iter())
            .map(|val| val.link.as_ref())
    }
}
