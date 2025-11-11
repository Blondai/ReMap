//! This module contains the implementation of [`Map`] and its methods.

#[allow(unused_imports)]
use crate::Row;

use std::{
    collections::HashSet,
    error::Error,
    fmt::{Display, Formatter},
    ops::Index,
    slice::Iter,
};

/// A struct containing a reordering of a [`Row`].
///
/// A [`Map`] defines the relationship between the source index (src) and the destination index (dest) of a column.
/// If the mapping is from source `i` to destination `j`, the value in [`Row`] at index `i` is moved to index `j` in the resulting [`Row`].
/// If the value of the [`Option`] at source `i` is the [`None`] variant the value of [`Row`] at this index is not part of the new [`Row`].
///
/// Per definitionem any [`Ok`] instance created with the provided initializers will be valid.
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    /// The source-destination-mapping.
    map: Vec<Option<usize>>,

    /// The largest destination index.
    max: usize,
}

impl Map {
    /// Creates a new [`Map`] instance based on a given `map`ping.
    ///
    /// This is the primary constructor.
    /// It takes a vector where the index represents the source field and
    /// the value represents the destination.
    /// A value of `Some(dest)` maps the source to a destination,
    /// while [`None`] indicates that the source field should be dropped.
    ///
    /// # Errors
    ///
    /// * [`MapError::DuplicateIndex`] - An destination index was used twice.
    /// * [`MapError::IndexTooLarge`] - The largest destination index is [`usize::MAX`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::{Map, MapError};
    /// // Swaps the first two columns and dropps the third.
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None];
    /// let map: Map = Map::new(map_vec).unwrap();
    /// assert_eq!(map.destination(0), Some(1));
    /// assert_eq!(map.destination(1), Some(0));
    /// assert_eq!(map.destination(2), None);
    ///
    /// // DuplicateIndex
    /// let map_vec: Vec<Option<usize>> = vec![Some(0), Some(0), None];
    /// let duplicate_index: MapError = Map::new(map_vec).err().unwrap();
    /// assert_eq!(duplicate_index, MapError::DuplicateIndex { dest: 0 });
    ///
    /// // IndexTooLarge
    /// let map_vec: Vec<Option<usize>> = vec![Some(usize::MAX), None];
    /// let index_too_large: MapError = Map::new(map_vec).err().unwrap();
    /// assert_eq!(index_too_large, MapError::IndexTooLarge);
    /// ```
    pub fn new(vec: Vec<Option<usize>>) -> Result<Self, MapError> {
        // Caching
        let max: usize = Self::max_dest_index(&vec);

        MapError::check_vec(&vec)?;
        MapError::check_index(max)?;

        Ok(Self { map: vec, max })
    }

    /// Creates the identity mapping.
    ///
    /// This maps each column to itself.
    /// Therefore, 0 -> 0, ..., length-1 -> length-1.
    ///
    /// Per definitionem this can not fail.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Map;
    /// let map: Map = Map::identity(3);
    /// assert_eq!(map.destination(0), Some(0));
    /// assert_eq!(map.destination(1), Some(1));
    /// assert_eq!(map.destination(2), Some(2));
    /// assert_eq!(map.destination(3), None);
    /// ```
    #[must_use]
    pub fn identity(length: usize) -> Self {
        // Can not fail
        let map: Vec<Option<usize>> = (0..length).map(Some).collect();
        let max: usize = length.saturating_sub(1);

        Self { map, max }
    }

    /// Creates a new [`Map`] instance with a sequential mapping.
    ///
    /// This maps `src_start` to `dest_start` and `src_start + 1` to `dest_start + 1` up until `src_end`.
    /// `src_end` is excluded.
    /// All values before `src_start` are marked with [`None`] and therefore dropped.
    ///
    /// # Errors
    ///
    /// * [`MapError::RangeError`] - The `src_start` is larger than `src_end`.
    /// * [`MapError::IndexTooLarge`] - The `dest_start + (src_end - src_start)` is larger than [`usize::MAX`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use remap::{Map, MapError};
    /// // Maps 2 -> 4, 3 -> 5, 4 -> 6, 5 -> 7, 6 -> 8
    /// let map: Map = Map::sequential(2, 7, 4).unwrap();
    /// assert_eq!(map.destination(0), None);
    /// assert_eq!(map.destination(4), Some(6));
    /// assert_eq!(map.destination(7), None);
    ///
    /// // Empty mapping
    /// let map: Map = Map::sequential(0, 0, 0).unwrap();
    /// assert_eq!(map.destination(0), None);
    /// assert_eq!(map.len(), 0);
    ///
    /// // RangeError
    /// let range_error: MapError = Map::sequential(7, 2, 4).err().unwrap();
    /// assert_eq!(range_error, MapError::RangeError { src_start: 7, src_end: 2 });
    ///
    /// // IndexTooLarge
    /// let index_too_large: MapError = Map::sequential(0, 5, usize::MAX - 1).err().unwrap();
    /// assert_eq!(index_too_large, MapError::IndexTooLarge);
    /// ```
    pub fn sequential(
        src_start: usize,
        src_end: usize,
        dest_start: usize,
    ) -> Result<Self, MapError> {
        MapError::check_range(src_start, src_end)?;
        MapError::check_overflow(src_start, src_end, dest_start)?;

        let max: usize = (dest_start + src_end.saturating_sub(src_start)).saturating_sub(1);

        let mut map: Vec<Option<usize>> = vec![None; src_end];

        let mut dest_index: usize = dest_start;

        // End is exclusive
        for src_index in src_start..src_end {
            map[src_index] = Some(dest_index);
            dest_index += 1;
        }

        Ok(Self { map, max })
    }

    /// Creates a tuple of two [`Map`]s that splices two [`Row`]s.
    ///
    /// This will take the part from `src_start_1` to `src_end_1` (excluding) of [`Row`] 1 and
    /// concatenate the part from `src_start_2` to `src_end_2` (excluding) of [`Row`] 2.
    ///
    /// This uses the [`Map::sequential`] method.
    ///
    /// # Errors
    ///
    /// * [`MapError::RangeError`] - The `src_start_*` is larger than `src_end_*`.
    /// * [`MapError::IndexTooLarge`] - The combined length of the two ranges exceeds [`usize::MAX`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::{Map, MapError};
    /// // Column 0 and 1 from row 1 and 1 and 2 from row 2.
    /// let (map_1, map_2): (Map, Map) = Map::splice(0, 2, 1, 3).unwrap();
    /// assert_eq!(map_1.destination(0), Some(0));
    /// assert_eq!(map_1.destination(1), Some(1));
    /// assert_eq!(map_1.destination(2), None);
    /// assert_eq!(map_2.destination(0), None);
    /// assert_eq!(map_2.destination(1), Some(2));
    /// assert_eq!(map_2.destination(2), Some(3));
    /// assert_eq!(map_2.destination(3), None);
    ///
    /// // RangeError
    /// let range_error: MapError = Map::splice(2, 0, 1, 3).err().unwrap();
    /// assert_eq!(range_error, MapError::RangeError { src_start: 2, src_end: 0 });
    ///
    /// // IndexTooLarge
    /// let index_too_large: MapError = Map::splice(0, usize::MAX, 0, 1).err().unwrap();
    /// assert_eq!(index_too_large, MapError::IndexTooLarge);
    /// ```
    pub fn splice(
        src_start_1: usize,
        src_end_1: usize,
        src_start_2: usize,
        src_end_2: usize,
    ) -> Result<(Self, Self), MapError> {
        let len_1: usize = src_end_1.saturating_sub(src_start_1);
        let len_2: usize = src_end_2.saturating_sub(src_start_2);

        // Prematurely triggers the `IndexTooLarge` error
        if len_1.checked_add(len_2).is_none() {
            return Err(MapError::IndexTooLarge);
        }

        let map_1: Map = Self::sequential(src_start_1, src_end_1, 0)?;
        let map_2: Map = Self::sequential(src_start_2, src_end_2, len_1)?;

        Ok((map_1, map_2))
    }

    /// Finds the destination index for a given source index.
    ///
    /// The `Some(usize)` is returned when `src` is in bounds and has a mapping.
    /// The [`None`] is returned when `src` is out of bounds or does not have a mapping.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Map;
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None];
    /// let map: Map = Map::new(map_vec).unwrap();
    /// assert_eq!(map.destination(0), Some(1));
    /// assert_eq!(map.destination(1), Some(0));
    /// assert_eq!(map.destination(2), None);
    /// ```
    #[inline]
    pub fn destination(&self, src: usize) -> Option<usize> {
        self.map.get(src).copied().flatten()
    }

    /// Returns the [`Map`] as an [`Iter`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Map;
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None];
    /// let map: Map = Map::new(map_vec).unwrap();
    /// let sum: usize = map.iter().filter_map(|dest| *dest).sum();
    /// assert_eq!(sum, 1);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, Option<usize>> {
        self.map.iter()
    }

    /// Returns the total number of source indices covered by the map.
    ///
    /// This represents the size of the source domain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Map;
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None];
    /// let map: Map = Map::new(map_vec).unwrap();
    /// assert_eq!(map.len(), 3);
    ///
    /// let map: Map = Map::sequential(0, 10, 0).unwrap();
    /// assert_eq!(map.len(), 10);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns a slice inside the [`Map`].
    ///
    /// Each element in the slice corresponds to a source index.
    /// The value is `Some(dest)` if a mapping exists to a destination index, or [`None`] if not.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Map;
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None];
    /// let map: Map = Map::new(map_vec).unwrap();
    /// assert_eq!(map.as_slice()[1], Some(0));
    ///
    /// let map: Map = Map::identity(3);
    /// assert_eq!(map.as_slice()[1], Some(1));
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[Option<usize>] {
        &self.map
    }

    /// Returns the largest destination index of a [`Map`] instance.
    ///
    /// The [`Map::max`] of an empty [`Map`] is `0`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Map;
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None];
    /// let map: Map = Map::new(map_vec).unwrap();
    /// assert_eq!(map.max(), 1);
    ///
    /// let map: Map = Map::identity(0);
    /// assert_eq!(map.max(), 0);
    /// ```
    #[inline]
    pub fn max(&self) -> usize {
        self.max
    }

    /// Calculates the largest destination index of a [`Vec<Option<usize>>`].
    ///
    /// This is used as a helper to populate the `max` field of the [`Map`] struct.
    ///
    /// This is set to `0` for the empty mapping.
    #[inline]
    fn max_dest_index(vec: &[Option<usize>]) -> usize {
        vec.iter()
            .filter_map(|dest_opt| *dest_opt)
            .max()
            .unwrap_or(0)
    }
}

impl Index<usize> for Map {
    type Output = Option<usize>;

    #[inline]
    fn index(&self, src: usize) -> &Self::Output {
        &self.map[src]
    }
}

impl Display for Map {
    fn fmt(&self, format: &mut Formatter<'_>) -> std::fmt::Result {
        for (src, dest) in self.map.iter().enumerate() {
            match *dest {
                Some(dest_index) => write!(format, "{} -> {}", src, dest_index)?,
                None => write!(format, "{} -> ∅", src)?,
            }

            // Adds commas
            if src < self.len().saturating_sub(1) {
                write!(format, ", ")?;
            }
        }

        Ok(())
    }
}

/// An enum for handling the errors involved in the creation of [`Map`] instances.
#[derive(Debug, Copy, Clone, PartialEq)]
#[non_exhaustive]
pub enum MapError {
    /// A destination occurs (at least) twice.
    DuplicateIndex { dest: usize },

    /// The destination index is too large.
    ///
    /// This would cause overflow when calculating the size of a destination row.
    ///
    /// In a real world scenario this will probably never happen,
    /// because a file with [`usize::MAX`] amount of columns would be ginormous.
    IndexTooLarge,

    /// The range of the destination indices are reversed.
    RangeError { src_start: usize, src_end: usize },
}

impl MapError {
    /// Checks a [`Vec`]tor for duplicate destinations.
    ///
    /// This creates a [`HashSet`].
    #[inline]
    fn check_vec(vec: &[Option<usize>]) -> Result<(), MapError> {
        let mut seen_indices: HashSet<usize> = HashSet::new();

        for dest in vec.iter().flatten() {
            if !seen_indices.insert(*dest) {
                return Err(MapError::DuplicateIndex { dest: *dest });
            }
        }

        Ok(())
    }

    /// Checks if the largest destination index is [`usize::MAX`].
    #[inline]
    fn check_index(max: usize) -> Result<(), MapError> {
        if max != usize::MAX {
            Ok(())
        } else {
            Err(MapError::IndexTooLarge)
        }
    }

    /// Checks if the range is correctly sorted.
    #[inline]
    fn check_range(src_start: usize, src_end: usize) -> Result<(), MapError> {
        if src_start <= src_end {
            Ok(())
        } else {
            Err(MapError::RangeError { src_start, src_end })
        }
    }

    /// Checks if the largest destination index is [`usize::MAX`]
    #[inline]
    fn check_overflow(src_start: usize, src_end: usize, dest_start: usize) -> Result<(), MapError> {
        if dest_start.checked_add(src_end - src_start).is_some() {
            Ok(())
        } else {
            Err(MapError::IndexTooLarge)
        }
    }
}

impl Display for MapError {
    fn fmt(&self, format: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MapError::DuplicateIndex { dest } => {
                write!(format, "Duplicate destination index: {}", dest)
            }
            MapError::IndexTooLarge => {
                write!(format, "Index ({}) is too large", usize::MAX)
            }
            MapError::RangeError { src_start, src_end } => {
                write!(
                    format,
                    "Source range error: start ({}) > end ({})",
                    src_start, src_end
                )
            }
        }
    }
}

impl Error for MapError {}
