#[cfg(not(feature = "std"))]
use alloc::collections::{BTreeMap, BTreeSet};
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::convert::TryFrom;
use core::fmt::{self, Display, Formatter};
#[cfg(feature = "std")]
use core::hash::Hash;
use ref_str::StaticRefStr;
#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq)]
/// Fixed-length non-zero hexadecimal identifier.
pub struct HexId<const N: usize>([u8; N]);

impl<const N: usize> fmt::Debug for HexId<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "HexId({})", self.as_str())
    }
}

impl<const N: usize> HexId<N> {
    /// Creates a validated hexadecimal identifier.
    pub const fn new(value: [u8; N]) -> Result<Self, &'static str> {
        let mut i = 0;
        let mut all_zeros = true;

        while i < N {
            let b = value[i];
            if !b.is_ascii_hexdigit() {
                return Err("invalid hex id: contains non-hex characters");
            }
            if b != b'0' {
                all_zeros = false;
            }
            i += 1;
        }

        if all_zeros {
            return Err("invalid hex id: cannot be all zeros");
        }

        Ok(Self(value))
    }

    /// Creates an unchecked identifier.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `value` is a valid hexadecimal identifier.
    pub const unsafe fn new_unchecked(value: [u8; N]) -> Self {
        Self(value)
    }

    /// Creates a validated hexadecimal identifier from a byte slice.
    ///
    /// If the slice length is less than N, the identifier is padded with '0' bytes at the beginning.
    /// For example, if N=4 and the input is "ab", the result will be "00ab".
    pub const fn from_bytes(value: &[u8]) -> Result<Self, &'static str> {
        let len = value.len();
        if len > N {
            return Err("invalid hex id: input too long");
        }
        if len == 0 {
            return Err("invalid hex id: input is empty");
        }

        let mut bytes = [b'0'; N];
        let mut i = 0;
        let offset = N - len;
        let mut all_zeros = true;

        while i < len {
            let b = value[i];
            if !b.is_ascii_hexdigit() {
                return Err("invalid hex id: contains non-hex characters");
            }
            if b != b'0' {
                all_zeros = false;
            }
            bytes[offset + i] = b;
            i += 1;
        }

        if all_zeros {
            return Err("invalid hex id: cannot be all zeros");
        }

        Ok(Self(bytes))
    }

    /// Creates a validated hexadecimal identifier from a str.
    pub const fn from_str(value: &str) -> Result<Self, &'static str> {
        let slice = value.as_bytes();
        if slice.len() != N {
            return Err("invalid hex id: length mismatch");
        }

        let mut bytes = [0u8; N];
        let mut i = 0;
        let mut all_zeros = true;

        while i < N {
            let b = slice[i];
            if !b.is_ascii_hexdigit() {
                return Err("invalid hex id: contains non-hex characters");
            }
            if b != b'0' {
                all_zeros = false;
            }
            bytes[i] = b;
            i += 1;
        }

        if all_zeros {
            return Err("invalid hex id: cannot be all zeros");
        }

        Ok(Self(bytes))
    }

    /// Creates an unchecked identifier from a str.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `value` is a valid hexadecimal identifier.
    pub const unsafe fn from_str_unchecked(value: &str) -> Self {
        let slice = value.as_bytes();
        let mut bytes = [0u8; N];
        let mut i = 0;
        while i < N {
            bytes[i] = slice[i];
            i += 1;
        }
        Self(bytes)
    }

    /// Returns the owned inner bytes.
    pub const fn into_inner(self) -> [u8; N] {
        self.0
    }

    /// Returns the identifier as a string slice.
    pub const fn as_str(&self) -> &str {
        // SAFETY: HexId is always validated to be ASCII hex digits during construction.
        unsafe { core::str::from_utf8_unchecked(&self.0) }
    }
}

impl<const N: usize> TryFrom<[u8; N]> for HexId<N> {
    type Error = &'static str;
    fn try_from(value: [u8; N]) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a, const N: usize> TryFrom<&'a str> for HexId<N> {
    type Error = &'static str;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

#[cfg(feature = "json")]
impl<const N: usize> serde::Serialize for HexId<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "json")]
impl<'de, const N: usize> serde::Deserialize<'de> for HexId<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct HexIdVisitor<const N: usize>;

        impl<'de, const N: usize> serde::de::Visitor<'de> for HexIdVisitor<N> {
            type Value = HexId<N>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a hex string of length {}", N)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                HexId::from_str(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(HexIdVisitor::<N>)
    }
}

impl<const N: usize> AsRef<str> for HexId<N> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const N: usize> core::ops::Deref for HexId<N> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<const N: usize> Display for HexId<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 16-byte trace id encoded as 32 lowercase hex chars.
pub type TraceId = HexId<32>;

#[cfg(feature = "otel")]
impl TryFrom<opentelemetry::TraceId> for TraceId {
    type Error = &'static str;
    fn try_from(value: opentelemetry::TraceId) -> Result<Self, Self::Error> {
        Self::from_bytes(value.to_bytes().as_ref())
    }
}

/// 8-byte span id encoded as 16 lowercase hex chars.
pub type SpanId = HexId<16>;

#[cfg(feature = "otel")]
impl TryFrom<opentelemetry::SpanId> for SpanId {
    type Error = &'static str;
    fn try_from(value: opentelemetry::SpanId) -> Result<Self, Self::Error> {
        Self::from_bytes(value.to_bytes().as_ref())
    }
}
/// Parent span id encoded as 16 lowercase hex chars.
pub type ParentSpanId = HexId<16>;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
pub struct TraceState(StaticRefStr);

impl TraceState {
    pub fn new(value: impl Into<StaticRefStr>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn as_static_ref(&self) -> &StaticRefStr {
        &self.0
    }

    pub fn into_inner(self) -> StaticRefStr {
        self.0
    }
}

impl From<StaticRefStr> for TraceState {
    fn from(value: StaticRefStr) -> Self {
        Self(value)
    }
}

impl AsRef<str> for TraceState {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::ops::Deref for TraceState {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Display for TraceState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "std")]
type FastMapImpl<K, V> = HashMap<K, V>;
#[cfg(not(feature = "std"))]
type FastMapImpl<K, V> = BTreeMap<K, V>;

#[cfg(feature = "std")]
type FastSetImpl<T> = HashSet<T>;
#[cfg(not(feature = "std"))]
type FastSetImpl<T> = BTreeSet<T>;

/// Fast map wrapper optimized for the current target environment.
#[derive(Debug, Clone)]
pub struct FastMap<K, V>(FastMapImpl<K, V>);

impl<K, V> Default for FastMap<K, V>
where
    FastMapImpl<K, V>: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> PartialEq for FastMap<K, V>
where
    FastMapImpl<K, V>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K, V> Eq for FastMap<K, V> where FastMapImpl<K, V>: Eq {}

#[cfg(feature = "json")]
impl<K, V> serde::Serialize for FastMap<K, V>
where
    FastMapImpl<K, V>: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "json")]
impl<'de, K, V> serde::Deserialize<'de> for FastMap<K, V>
where
    FastMapImpl<K, V>: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        FastMapImpl::<K, V>::deserialize(deserializer).map(Self)
    }
}

impl<K, V> FastMap<K, V> {
    /// Creates an empty fast map.
    pub fn new() -> Self {
        Self(FastMapImpl::default())
    }

    /// Returns true if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Removes all key-value pairs from the map.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns an iterator over key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter()
    }

    /// Returns a mutable iterator over key-value pairs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.0.iter_mut()
    }
}

#[cfg(feature = "std")]
impl<K, V> FastMap<K, V>
where
    K: Eq + Hash + Ord,
{
    /// Returns all entries sorted by key.
    pub fn sorted_entries(&self) -> Vec<(&K, &V)> {
        let mut entries: Vec<_> = self.0.iter().collect();
        entries.sort_by_key(|(left, _)| *left);
        entries
    }

    /// Consumes the map and returns all entries sorted by key.
    pub fn into_sorted_entries(self) -> Vec<(K, V)> {
        let mut entries: Vec<_> = self.0.into_iter().collect();
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));
        entries
    }
}

#[cfg(not(feature = "std"))]
impl<K, V> FastMap<K, V>
where
    K: Ord,
{
    /// Returns all entries sorted by key.
    pub fn sorted_entries(&self) -> Vec<(&K, &V)> {
        self.0.iter().collect()
    }

    /// Consumes the map and returns all entries sorted by key.
    pub fn into_sorted_entries(self) -> Vec<(K, V)> {
        self.0.into_iter().collect()
    }
}

#[cfg(feature = "std")]
impl<K, V> FastMap<K, V> {
    /// Creates a fast map with the requested capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
    }
}

#[cfg(not(feature = "std"))]
impl<K, V> FastMap<K, V> {
    /// Creates a fast map with the requested capacity hint.
    pub fn with_capacity(_: usize) -> Self {
        Self(BTreeMap::new())
    }
}

#[cfg(feature = "std")]
impl<K, V> FastMap<K, V>
where
    K: Eq + Hash,
{
    /// Inserts a key-value pair into the map.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    /// Returns a shared reference to the value corresponding to the key.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.0.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.0.get_mut(key)
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.0.contains_key(key)
    }

    /// Removes a key from the map, returning the value if the key was in the map.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.0.remove(key)
    }
}

#[cfg(not(feature = "std"))]
impl<K, V> FastMap<K, V>
where
    K: Ord,
{
    /// Inserts a key-value pair into the map.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    /// Returns a shared reference to the value corresponding to the key.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.0.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.0.get_mut(key)
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.0.contains_key(key)
    }

    /// Removes a key from the map, returning the value if the key was in the map.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.0.remove(key)
    }
}

impl<K, V> Extend<(K, V)> for FastMap<K, V>
where
    FastMapImpl<K, V>: Extend<(K, V)>,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<K, V> FromIterator<(K, V)> for FastMap<K, V>
where
    FastMapImpl<K, V>: FromIterator<(K, V)>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<K, V> IntoIterator for FastMap<K, V> {
    type Item = (K, V);
    type IntoIter = <FastMapImpl<K, V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K, V> IntoIterator for &'a FastMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = <&'a FastMapImpl<K, V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut FastMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = <&'a mut FastMapImpl<K, V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

/// Fast set wrapper optimized for the current target environment.
#[derive(Debug, Clone)]
pub struct FastSet<T>(FastSetImpl<T>);

impl<T> Default for FastSet<T>
where
    FastSetImpl<T>: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PartialEq for FastSet<T>
where
    FastSetImpl<T>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for FastSet<T> where FastSetImpl<T>: Eq {}

#[cfg(feature = "json")]
impl<T> serde::Serialize for FastSet<T>
where
    FastSetImpl<T>: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "json")]
impl<'de, T> serde::Deserialize<'de> for FastSet<T>
where
    FastSetImpl<T>: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        FastSetImpl::<T>::deserialize(deserializer).map(Self)
    }
}

impl<T> FastSet<T> {
    /// Creates an empty fast set.
    pub fn new() -> Self {
        Self(FastSetImpl::default())
    }

    /// Returns true if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Removes all elements from the set.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns an iterator over all values in the set.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }
}

#[cfg(feature = "std")]
impl<T> FastSet<T> {
    /// Creates a fast set with the requested capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashSet::with_capacity(capacity))
    }
}

#[cfg(not(feature = "std"))]
impl<T> FastSet<T> {
    /// Creates a fast set with the requested capacity hint.
    pub fn with_capacity(_: usize) -> Self {
        Self(BTreeSet::new())
    }
}

#[cfg(feature = "std")]
impl<T> FastSet<T>
where
    T: Eq + Hash,
{
    /// Adds a value to the set. Returns true if the value was not present.
    pub fn insert(&mut self, value: T) -> bool {
        self.0.insert(value)
    }

    /// Returns true if the set contains the value.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.0.contains(value)
    }

    /// Removes a value from the set. Returns true if the value existed.
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.0.remove(value)
    }
}

#[cfg(not(feature = "std"))]
impl<T> FastSet<T>
where
    T: Ord,
{
    /// Adds a value to the set. Returns true if the value was not present.
    pub fn insert(&mut self, value: T) -> bool {
        self.0.insert(value)
    }

    /// Returns true if the set contains the value.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.0.contains(value)
    }

    /// Removes a value from the set. Returns true if the value existed.
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.0.remove(value)
    }
}

impl<T> Extend<T> for FastSet<T>
where
    FastSetImpl<T>: Extend<T>,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<T> FromIterator<T> for FastSet<T>
where
    FastSetImpl<T>: FromIterator<T>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T> IntoIterator for FastSet<T> {
    type Item = T;
    type IntoIter = <FastSetImpl<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a FastSet<T> {
    type Item = &'a T;
    type IntoIter = <&'a FastSetImpl<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
