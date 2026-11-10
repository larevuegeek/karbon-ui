pub(super) use crate::ui::scene::Image;
pub(super) use std::{collections::HashMap, hash::Hash, sync::Arc};

/// Snapshot of an [`ImageCache`]'s counters, returned by [`ImageCache::stats`].
#[derive(Debug, Clone, Copy)]
pub struct ImageCacheStats {
    /// Number of cached images.
    pub entries: usize,
    /// Bytes of pixel storage currently retained (allocated capacity).
    pub bytes: usize,
    /// Maximum number of bytes the cache may retain.
    pub budget: usize,
    /// Number of [`get`](ImageCache::get) calls that found their key.
    pub hits: u64,
    /// Number of [`get`](ImageCache::get) calls that missed.
    pub misses: u64,
    /// Number of entries evicted to stay within the budget.
    pub evictions: u64,
}
pub(super) struct Entry {
    pub(super) image: Arc<Image>,
    pub(super) used: u64,
}

/// Least-recently-used cache of decoded images, bounded by a byte budget.
///
/// The budget counts the allocated capacity of each image's pixel buffer. Eviction only
/// drops the cache's reference: a scene may still keep its `Arc` alive.
pub struct ImageCache<K> {
    pub(super) entries: HashMap<K, Entry>,
    pub(super) bytes: usize,
    pub(super) budget: usize,
    pub(super) clock: u64,
    pub(super) hits: u64,
    pub(super) misses: u64,
    pub(super) evictions: u64,
}
impl<K: Eq + Hash + Clone> ImageCache<K> {
    /// Create an empty cache that retains at most `budget` bytes of pixels.
    pub fn new(budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            bytes: 0,
            budget,
            clock: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }
    pub(super) fn tick(&mut self) -> u64 {
        if self.clock == u64::MAX {
            let mut order: Vec<_> = self
                .entries
                .iter()
                .map(|(key, entry)| (key.clone(), entry.used))
                .collect();
            order.sort_unstable_by_key(|(_, used)| *used);
            for (index, (key, _)) in order.into_iter().enumerate() {
                self.entries.get_mut(&key).unwrap().used = index as u64;
            }
            self.clock = self.entries.len() as u64;
        }
        self.clock += 1;
        self.clock
    }
    /// Return the image for `key` and mark it as most recently used.
    pub fn get(&mut self, key: &K) -> Option<Arc<Image>> {
        let used = self.tick();
        if let Some(entry) = self.entries.get_mut(key) {
            entry.used = used;
            self.hits = self.hits.saturating_add(1);
            Some(Arc::clone(&entry.image))
        } else {
            self.misses = self.misses.saturating_add(1);
            None
        }
    }
    /// Check for `key` without updating recency or the hit/miss counters.
    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }
    /// Insert or replace the image for `key`, evicting the least recently used entries
    /// as needed.
    ///
    /// Returns `Ok(false)` without evicting anything when the image alone exceeds the
    /// budget.
    ///
    /// # Errors
    ///
    /// [`ImageError::InvalidPixels`](crate::ImageError::InvalidPixels) when the image is
    /// empty or its pixel buffer does not hold `width * height * 4` bytes.
    pub fn insert(&mut self, key: K, image: Arc<Image>) -> crate::Result<bool> {
        let expected = (image.width as usize)
            .checked_mul(image.height as usize)
            .and_then(|n| n.checked_mul(4));
        if image.width == 0 || image.height == 0 || expected != Some(image.pixels.len()) {
            return Err(crate::ImageError::InvalidPixels.into());
        }
        let size = image.pixels.capacity();
        if size > self.budget {
            return Ok(false);
        }
        self.remove(&key);
        while self.bytes > self.budget - size {
            self.evict_oldest();
        }
        let used = self.tick();
        self.bytes += size;
        self.entries.insert(key, Entry { image, used });
        Ok(true)
    }
    /// Remove and return the image for `key`, if cached.
    pub fn remove(&mut self, key: &K) -> Option<Arc<Image>> {
        let entry = self.entries.remove(key)?;
        self.bytes -= entry.image.pixels.capacity();
        Some(entry.image)
    }
    pub(super) fn evict_oldest(&mut self) {
        if let Some(key) = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.used)
            .map(|(key, _)| key.clone())
        {
            self.remove(&key);
            self.evictions = self.evictions.saturating_add(1);
        }
    }
    /// Change the byte budget, evicting least recently used entries until it fits.
    pub fn set_budget(&mut self, budget: usize) {
        self.budget = budget;
        while self.bytes > budget {
            self.evict_oldest();
        }
    }
    /// Remove every entry. Counters are kept.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
    /// Current size, budget and counters.
    pub fn stats(&self) -> ImageCacheStats {
        ImageCacheStats {
            entries: self.entries.len(),
            bytes: self.bytes,
            budget: self.budget,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image(value: u8) -> Arc<Image> {
        Arc::new(Image::new(2, 2, vec![value; 16]).unwrap())
    }
    #[test]
    fn cache_evicts_least_recent_retains_hits_and_accounts_replacements() {
        let mut cache = ImageCache::new(32);
        cache.insert(1, image(10)).unwrap();
        cache.insert(2, image(20)).unwrap();
        cache.get(&1).unwrap();
        cache.insert(3, image(30)).unwrap();
        assert!(!cache.contains(&2));
        assert!(cache.contains(&1));
        assert_eq!(cache.stats().bytes, 32);
        assert_eq!(cache.stats().evictions, 1);
        assert_eq!(cache.stats().hits, 1);
        cache.insert(1, image(40)).unwrap();
        assert_eq!(cache.get(&1).unwrap().pixels[0], 40);
        assert_eq!(cache.stats().entries, 2);
        assert!(cache.get(&99).is_none());
        assert_eq!(cache.stats().misses, 1);
    }
    #[test]
    fn budget_counts_allocated_capacity_and_rejects_large_images_without_losing_entries() {
        let mut cache = ImageCache::new(16);
        cache.insert(1, image(10)).unwrap();
        let mut pixels = Vec::with_capacity(64);
        pixels.resize(16, 0);
        let large = Arc::new(Image::new(2, 2, pixels).unwrap());
        assert!(!cache.insert(2, large).unwrap());
        assert!(cache.contains(&1));
        assert!(matches!(
            cache.insert(
                3,
                Arc::new(Image {
                    width: 3,
                    height: 2,
                    pixels: vec![0; 4]
                })
            ),
            Err(crate::Error::Image(crate::ImageError::InvalidPixels))
        ));
        cache.set_budget(0);
        assert_eq!(cache.stats().bytes, 0);
        assert_eq!(cache.stats().entries, 0);
    }
    #[test]
    fn eviction_releases_ownership_but_a_visible_scene_can_pin_an_image() {
        let mut cache = ImageCache::new(16);
        let original = image(10);
        let weak = Arc::downgrade(&original);
        cache.insert(1, original).unwrap();
        let visible = cache.get(&1).unwrap();
        cache.insert(2, image(20)).unwrap();
        assert!(weak.upgrade().is_some());
        drop(visible);
        assert!(weak.upgrade().is_none());
        cache.clear();
        assert_eq!(cache.stats().bytes, 0);
    }
    #[test]
    fn timestamp_overflow_preserves_recency_order() {
        let mut cache = ImageCache::new(32);
        cache.insert(1, image(10)).unwrap();
        cache.insert(2, image(20)).unwrap();
        cache.clock = u64::MAX;
        cache.get(&1);
        cache.insert(3, image(30)).unwrap();
        assert!(!cache.contains(&2));
        assert!(cache.contains(&1));
    }
}
