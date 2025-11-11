//! This module contains the implementation of [`Row`] and its methods.

use crate::Map;

use std::{
    collections::HashSet,
    error::Error,
    fmt::{Display, Formatter},
    slice::Iter,
};

/// Represents an ordered collection of fields, like a single row of csv table.
///
/// Using the [`Row::combine`] or [`Row::reorder`] this can be reordered using a [`Map`].
///
/// This is a wrapper around [`Vec<T>`].
#[derive(Clone, Debug, PartialEq)]
pub struct Row<T> {
    /// The [`Vec<T>`] containing the data of the row.
    row: Vec<T>,
}

impl<T> Row<T> {
    /// Creates a new [`Row`] based on a [`Vec<T>`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Row;
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row<String> = Row::new(row_vec);
    /// assert_eq!(row[0], "a");
    /// assert_eq!(row[1], "b");
    /// assert_eq!(row[2], "c");
    /// ```
    #[inline]
    #[must_use]
    pub fn new(row: Vec<T>) -> Self {
        Self { row }
    }

    /// Combines a pair of [`Row`]s with a pair of [`Map`]s to return a new [`Row`] **of references** ordered based one the [`Map`]s.
    ///
    /// This is a zero-copy version of [`Row::combine`].
    ///
    /// This is the primary function for splicing data from multiple sources.
    /// It uses `map_1` to select and place fields from `row_1` and
    /// `map_2` to select and place fields from `row_2`.
    ///
    /// Any destination index that is not targeted by either map will be filled with a `fallback` value.
    ///
    /// # Errors
    ///
    /// * [`CombinationError::LengthError`] - If `map_1` is longer than `row_1` or if `map_2` is longer than `row_2`.
    /// This prevents out-of-bounds access.
    /// * [`CombinationError::CollisionError`] - If `map_1` and `map_2` both attempt to map to the same destination index.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::{CombinationError, Map, Row};
    /// // "c","","a","","2"
    /// let vec_1: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row_1: Row<String> = Row::new(vec_1);
    /// let vec_2: Vec<String> = vec!["1", "2", "3"].into_iter().map(String::from).collect();
    /// let row_2: Row<String> = Row::new(vec_2);
    ///
    /// let vec_1: Vec<Option<usize>> = vec![Some(2), None, Some(0)];
    /// let map_1: Map = Map::new(vec_1).unwrap();
    /// let vec_2: Vec<Option<usize>> = vec![None, Some(4), None];
    /// let map_2: Map = Map::new(vec_2).unwrap();
    ///
    /// let fallback: String = String::from("");
    /// let row: Row<&String> = Row::combine_ref(&row_1, &map_1, &row_2, &map_2, &fallback).unwrap();
    /// assert_eq!(row[0], "c");
    /// assert_eq!(row[1], "");
    /// assert_eq!(row[2], "a");
    /// assert_eq!(row[3], "");
    /// assert_eq!(row[4], "2");
    ///
    /// // CollisionError
    /// let vec_2: Vec<Option<usize>> = vec![None, Some(2), None];
    /// let map_2: Map = Map::new(vec_2).unwrap();
    /// let collision_error: CombinationError = Row::combine_ref(&row_1, &map_1, &row_2, &map_2, &fallback).err().unwrap();
    /// assert_eq!(collision_error, CombinationError::CollisionError { dest: 2 });
    ///
    /// // LengthError
    /// let vec_2: Vec<Option<usize>> = vec![None, Some(4), None, Some(1)];
    /// let map_2: Map = Map::new(vec_2).unwrap();
    /// let length_error: CombinationError = Row::combine_ref(&row_1, &map_1, &row_2, &map_2, &fallback).err().unwrap();
    /// assert_eq!(length_error, CombinationError::LengthError { map_len: 4, row_len: 3 });
    /// ```
    pub fn combine_ref<'a>(
        row_1: &'a Row<T>,
        map_1: &Map,
        row_2: &'a Row<T>,
        map_2: &Map,
        fallback: &'a T,
    ) -> Result<Row<&'a T>, CombinationError> {
        CombinationError::check_lengths(row_1, map_1)?;
        CombinationError::check_lengths(row_2, map_2)?;
        CombinationError::check_collision(map_1, map_2)?;

        let dest_len: usize = usize::max(map_1.max(), map_2.max()) + 1;
        let mut dest_buffer: Vec<Option<&T>> = vec![None; dest_len];

        Self::apply_map(map_1, row_1, &mut dest_buffer);
        Self::apply_map(map_2, row_2, &mut dest_buffer);

        Ok(Row::new(Self::buffer_to_vec_ref(dest_buffer, fallback)))
    }

    /// Combines a [`Row`] with a [`Map`] to return a new [`Row`] **of references** ordered based one the [`Map`].
    ///
    /// This is a zero-copy version of [`Row::reorder`].
    ///
    /// This allows reordering of a [`Row`].
    /// This also includes dropping specific columns.
    ///
    /// Any destination index that is not targeted by either map will be filled with a `fallback` value.
    ///
    /// # Errors
    ///
    /// * [`CombinationError::LengthError`] - If `map` is longer than `row` (`self`).
    /// This prevents out-of-bounds access.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::{CombinationError, Map, Row};
    /// // "b","a"
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row<String> = Row::new(row_vec);
    ///
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None];
    /// let map: Map = Map::new(map_vec).unwrap();
    ///
    /// let fallback: String = String::from("");
    /// let reordering: Row<&String> = row.reorder_ref(&map, &fallback).unwrap();
    /// assert_eq!(reordering[0], "b");
    /// assert_eq!(reordering[1], "a");
    /// assert_eq!(reordering.len(), 2);
    ///
    /// // LengthError
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None, Some(2)];
    /// let map: Map = Map::new(map_vec).unwrap();
    /// let length_error: CombinationError = row.reorder_ref(&map, &fallback).err().unwrap();
    /// assert_eq!(length_error, CombinationError::LengthError { map_len: 4, row_len: 3 });
    /// ```
    pub fn reorder_ref<'a>(
        &'a self,
        map: &Map,
        fallback: &'a T,
    ) -> Result<Row<&'a T>, CombinationError> {
        CombinationError::check_lengths(self, map)?;

        let dest_len: usize = map.max() + 1;

        let mut dest_buffer: Vec<Option<&T>> = vec![None; dest_len];

        Self::apply_map(map, self, &mut dest_buffer);

        Ok(Row::new(Self::buffer_to_vec_ref(dest_buffer, fallback)))
    }

    /// Returns the [`Row`] as an [`Iter`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Row;
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row<String> = Row::new(row_vec);
    /// let mut iter: std::slice::Iter<String> = row.iter();
    /// assert_eq!(iter.next(), Some(&"a".to_string()));
    /// assert_eq!(iter.next(), Some(&"b".to_string()));
    /// assert_eq!(iter.next(), Some(&"c".to_string()));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.row.iter()
    }

    /// Returns the amount of columns.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Row;
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row<String> = Row::new(row_vec);
    /// assert_eq!(row.len(), 3);
    ///
    /// let row_vec: Vec<String> = Vec::new();
    /// let row: Row<String> = Row::new(row_vec);
    /// assert_eq!(row.len(), 0);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.row.len()
    }

    /// Returns a slice of the [`Vec<T>`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Row;
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row<String> = Row::new(row_vec);
    /// assert_eq!(row.as_slice()[0], "a");
    /// assert_eq!(row.as_slice()[1], "b");
    /// assert_eq!(row.as_slice()[2], "c");
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.row
    }

    /// Converts a buffer of [`Option<&T>`] into a vector of owned [`T`]s.
    ///
    /// This is a zero-copy version of [`Row::buffer_to_vec`].
    ///
    /// This internal helper function is used by [`Row::combine_ref`] and [`Row::reorder_ref`] to finalize the construction of a new [`Row`].
    #[inline]
    fn buffer_to_vec_ref<'a>(dest_buffer: Vec<Option<&'a T>>, fallback: &'a T) -> Vec<&'a T> {
        dest_buffer
            .into_iter()
            .map(|opt| opt.unwrap_or(fallback))
            .collect()
    }

    /// Applies a [`Map`] to a [`Row`] and writes the result to a buffer.
    ///
    /// # Panics
    ///
    /// This function performs no validation and is designed for internal use.
    /// The caller **must** ensure that the inputs are valid to prevent panics:
    /// - It will panic if `map.len()` is greater than `src_row.len()`.
    /// - It will panic if any destination index in the `map` is out of bounds for `dest_buffer`.
    #[inline]
    fn apply_map<'a>(map: &Map, src_row: &'a Row<T>, dest_buffer: &mut Vec<Option<&'a T>>) {
        for (src, dest_opt) in map.iter().enumerate() {
            if let Some(dest) = *dest_opt {
                dest_buffer[dest] = Some(&src_row[src]);
            }
        }
    }
}

impl<T: Clone + Default> Row<T> {
    /// Combines a pair of [`Row`]s with a pair of [`Map`]s to return a new [`Row`] ordered based one the [`Map`]s.
    ///
    /// This is the primary function for splicing data from multiple sources.
    /// It uses `map_1` to select and place fields from `row_1` and
    /// `map_2` to select and place fields from `row_2`.
    ///
    /// Any destination index that is not targeted by either map will be filled with a [`Default`] value.
    ///
    /// # Errors
    ///
    /// * [`CombinationError::LengthError`] - If `map_1` is longer than `row_1` or if `map_2` is longer than `row_2`.
    /// This prevents out-of-bounds access.
    /// * [`CombinationError::CollisionError`] - If `map_1` and `map_2` both attempt to map to the same destination index.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::{CombinationError, Map, Row};
    /// // "c","","a","","2"
    /// let vec_1: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row_1: Row<String> = Row::new(vec_1);
    /// let vec_2: Vec<String> = vec!["1", "2", "3"].into_iter().map(String::from).collect();
    /// let row_2: Row<String> = Row::new(vec_2);
    ///
    /// let vec_1: Vec<Option<usize>> = vec![Some(2), None, Some(0)];
    /// let map_1: Map = Map::new(vec_1).unwrap();
    /// let vec_2: Vec<Option<usize>> = vec![None, Some(4), None];
    /// let map_2: Map = Map::new(vec_2).unwrap();
    ///
    /// let row: Row<String> = Row::combine(&row_1, &map_1, &row_2, &map_2).unwrap();
    /// assert_eq!(row[0], "c");
    /// assert_eq!(row[1], "");
    /// assert_eq!(row[2], "a");
    /// assert_eq!(row[3], "");
    /// assert_eq!(row[4], "2");
    ///
    /// // CollisionError
    /// let vec_2: Vec<Option<usize>> = vec![None, Some(2), None];
    /// let map_2: Map = Map::new(vec_2).unwrap();
    /// let collision_error: CombinationError = Row::combine(&row_1, &map_1, &row_2, &map_2).err().unwrap();
    /// assert_eq!(collision_error, CombinationError::CollisionError { dest: 2 });
    ///
    /// // LengthError
    /// let vec_2: Vec<Option<usize>> = vec![None, Some(4), None, Some(1)];
    /// let map_2: Map = Map::new(vec_2).unwrap();
    /// let length_error: CombinationError = Row::combine(&row_1, &map_1, &row_2, &map_2).err().unwrap();
    /// assert_eq!(length_error, CombinationError::LengthError { map_len: 4, row_len: 3 });
    /// ```
    pub fn combine(
        row_1: &Row<T>,
        map_1: &Map,
        row_2: &Row<T>,
        map_2: &Map,
    ) -> Result<Self, CombinationError> {
        CombinationError::check_lengths(row_1, map_1)?;
        CombinationError::check_lengths(row_2, map_2)?;
        CombinationError::check_collision(map_1, map_2)?;

        let dest_len: usize = usize::max(map_1.max(), map_2.max()) + 1;
        let mut dest_buffer: Vec<Option<&T>> = vec![None; dest_len];

        Self::apply_map(map_1, row_1, &mut dest_buffer);
        Self::apply_map(map_2, row_2, &mut dest_buffer);

        Ok(Row::new(Self::buffer_to_vec(dest_buffer)))
    }

    /// Combines a [`Row`] with a [`Map`] to return a new [`Row`] ordered based one the [`Map`].
    ///
    /// This allows reordering of a [`Row`].
    /// This also includes dropping specific columns.
    ///
    /// Any destination index that is not targeted by either map will be filled with a [`Default`] value.
    ///
    /// # Errors
    ///
    /// * [`CombinationError::LengthError`] - If `map` is longer than `row` (`self`).
    /// This prevents out-of-bounds access.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::{CombinationError, Map, Row};
    /// // "b","a"
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row<String> = Row::new(row_vec);
    ///
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None];
    /// let map: Map = Map::new(map_vec).unwrap();
    ///
    /// let reordering: Row<String> = row.reorder(&map).unwrap();
    /// assert_eq!(reordering[0], "b");
    /// assert_eq!(reordering[1], "a");
    /// assert_eq!(reordering.len(), 2);
    ///
    /// // LengthError
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None, Some(2)];
    /// let map: Map = Map::new(map_vec).unwrap();
    /// let length_error: CombinationError = row.reorder(&map).err().unwrap();
    /// assert_eq!(length_error, CombinationError::LengthError { map_len: 4, row_len: 3 });
    /// ```
    pub fn reorder(&self, map: &Map) -> Result<Self, CombinationError> {
        CombinationError::check_lengths(self, map)?;

        let dest_len: usize = map.max() + 1;

        let mut dest_buffer: Vec<Option<&T>> = vec![None; dest_len];

        Self::apply_map(map, self, &mut dest_buffer);

        Ok(Row::new(Self::buffer_to_vec(dest_buffer)))
    }

    /// Converts a buffer of [`Option<&T>`] into a vector of owned [`T`]s.
    ///
    /// This internal helper function is used by [`Row::combine`] and [`Row::reorder`] to finalize the construction of a new [`Row`].
    ///
    /// This approach minimizes new memory allocations by cloning existing data whenever possible.
    #[inline]
    fn buffer_to_vec(dest_buffer: Vec<Option<&T>>) -> Vec<T> {
        dest_buffer
            .into_iter()
            .map(|dest_opt| match dest_opt {
                Some(t) => t.clone(), // Clone the reference
                None => T::default(), // Allocate fallbacks
            })
            .collect()
    }
}

impl<'a, T: Clone> Row<&'a T> {
    /// Creates an owned [`Row<T>`] from a [`Row<&T>`] of references by cloning each element.
    #[allow(clippy::wrong_self_convention)]
    #[must_use]
    pub fn into_owned(&self) -> Row<T> {
        let owned: Vec<T> = self.row.iter().map(|&t| t.clone()).collect();

        Row::new(owned)
    }
}

impl<T> std::ops::Index<usize> for Row<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.row[index]
    }
}

impl<T: Display> Display for Row<T> {
    fn fmt(&self, format: &mut Formatter<'_>) -> std::fmt::Result {
        for (index, el) in self.row.iter().enumerate() {
            write!(format, "{}", el)?;

            // Adds commas
            if index < self.len().saturating_sub(1) {
                write!(format, ", ")?;
            }
        }
        Ok(())
    }
}

/// An enum for handling the errors involved in the combination of a [`Row`] and a [`Map`].
#[derive(Debug, Copy, Clone, PartialEq)]
#[non_exhaustive]
pub enum CombinationError {
    /// Both [`Map`]s want to assign a value to the same destination index.
    CollisionError { dest: usize },

    /// The length of the [`Map`] is larger than the length of the [`Row`].
    LengthError { map_len: usize, row_len: usize },
}

impl CombinationError {
    /// Checks two [`Map`]s if they have a destination index in common.
    ///
    /// As per definitionem every [`Map`] contains each destination index at most once,
    /// a [`HashSet`]
    fn check_collision(map_1: &Map, map_2: &Map) -> Result<(), CombinationError> {
        // map_1 per definitionem only contain each index at most once
        let seen_indices: HashSet<usize> = map_1.iter().filter_map(|dest_opt| *dest_opt).collect();

        for dest in map_2.iter().flatten() {
            if seen_indices.contains(dest) {
                return Err(CombinationError::CollisionError { dest: *dest });
            }
        }

        Ok(())
    }

    /// Checks if the length of a [`Row`] is at least as big as the length of the corresponding [`Map`].
    #[inline]
    fn check_lengths<T>(row: &Row<T>, map: &Map) -> Result<(), CombinationError> {
        if map.len() <= row.len() {
            Ok(())
        } else {
            Err(Self::LengthError {
                map_len: map.len(),
                row_len: row.len(),
            })
        }
    }
}

impl Display for CombinationError {
    fn fmt(&self, format: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CollisionError { dest } => {
                write!(format, "Collision at destination index {}", dest)
            }
            Self::LengthError { row_len, map_len } => write!(
                format,
                "Map of length {} is longer than row of length {}",
                map_len, row_len
            ),
        }
    }
}

impl Error for CombinationError {}
