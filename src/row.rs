//! This module contains the implementation of [`Row`] and its methods.

use crate::Map;

use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
    slice::Iter,
};

/// Represents an ordered collection of string fields, like a single row of csv table.
///
/// Using the [`Row::combine`] or [`Row::reorder`] this can be reordered using a [`Map`].
///
/// This is a wrapper around [`Vec<String>`].
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The [`Vec<String>`] containing the data of the row.
    row: Vec<String>,
}

impl Row {
    /// The value when there is no mapping to a column.
    pub const FALLBACK: &'static str = "";

    /// Creates a new [`Row`] based on a [`Vec<String>`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Row;
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row = Row::new(row_vec);
    /// assert_eq!(row[0], "a");
    /// assert_eq!(row[1], "b");
    /// assert_eq!(row[2], "c");
    /// ```
    pub fn new(row: Vec<String>) -> Self {
        Self { row }
    }

    /// Combines a pair of [`Row`]s with a pair of [`Map`]s to return a new [`Row`] ordered based one the [`Map`]s.
    ///
    /// This is the primary function for splicing data from multiple sources.
    /// It uses `map_1` to select and place fields from `row_1` and
    /// `map_2` to select and place fields from `row_2`.
    ///
    /// Any destination index that is not targeted by either map will be filled with a [`Row::FALLBACK`] value.
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
    /// let row_1: Row = Row::new(vec_1);
    /// let vec_2: Vec<String> = vec!["1", "2", "3"].into_iter().map(String::from).collect();
    /// let row_2: Row = Row::new(vec_2);
    ///
    /// let vec_1: Vec<Option<usize>> = vec![Some(2), None, Some(0)];
    /// let map_1: Map = Map::new(vec_1).unwrap();
    /// let vec_2: Vec<Option<usize>> = vec![None, Some(4), None];
    /// let map_2: Map = Map::new(vec_2).unwrap();
    ///
    /// let row: Row = Row::combine(&row_1, &map_1, &row_2, &map_2).unwrap();
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
        row_1: &Row,
        map_1: &Map,
        row_2: &Row,
        map_2: &Map,
    ) -> Result<Self, CombinationError> {
        CombinationError::check_lengths(row_1, map_1)?;
        CombinationError::check_lengths(row_2, map_2)?;
        CombinationError::check_collision(map_1, map_2)?;

        let dest_len: usize = usize::max(map_1.max(), map_2.max()) + 1;
        let mut dest_buffer: Vec<Option<&str>> = vec![None; dest_len];

        Self::apply_map(map_1, row_1, &mut dest_buffer);
        Self::apply_map(map_2, row_2, &mut dest_buffer);

        Ok(Row::new(Self::buffer_to_vec(dest_buffer)))
    }

    /// Combines a [`Row`] with a [`Map`] to return a new [`Row`] ordered based one the [`Map`].
    ///
    /// This allows reordering of a [`Row`].
    /// This also includes dropping specific columns.
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
    /// let row: Row = Row::new(row_vec);
    ///
    /// let map_vec: Vec<Option<usize>> = vec![Some(1), Some(0), None];
    /// let map: Map = Map::new(map_vec).unwrap();
    ///
    /// let reordering: Row = row.reorder(&map).unwrap();
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

        let mut dest_buffer: Vec<Option<&str>> = vec![None; dest_len];

        Self::apply_map(map, self, &mut dest_buffer);

        Ok(Row::new(Self::buffer_to_vec(dest_buffer)))
    }

    /// Returns the [`Row`] as an [`Iter`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Row;
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row = Row::new(row_vec);
    /// let mut iter: std::slice::Iter<String> = row.iter();
    /// assert_eq!(iter.next(), Some(&"a".to_string()));
    /// assert_eq!(iter.next(), Some(&"b".to_string()));
    /// assert_eq!(iter.next(), Some(&"c".to_string()));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn iter(&self) -> Iter<'_, String> {
        self.row.iter()
    }

    /// Returns the amount of columns.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Row;
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row = Row::new(row_vec);
    /// assert_eq!(row.len(), 3);
    ///
    /// let row_vec: Vec<String> = Vec::new();
    /// let row: Row = Row::new(row_vec);
    /// assert_eq!(row.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.row.len()
    }

    /// Returns a slice of the `Vec<String>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use remap::Row;
    /// let row_vec: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    /// let row: Row = Row::new(row_vec);
    /// assert_eq!(row.as_slice()[0], "a");
    /// assert_eq!(row.as_slice()[1], "b");
    /// assert_eq!(row.as_slice()[2], "c");
    /// ```
    pub fn as_slice(&self) -> &[String] {
        &self.row
    }

    /// Converts a buffer of [`Option<&str>`] into a vector of owned [`String`]s.
    ///
    /// This internal helper function is used by [`Row::combine`] and [`Row::reorder`] to finalize the construction of a new `Row`.
    ///
    /// This approach minimizes new memory allocations by cloning existing string data whenever possible.
    fn buffer_to_vec(dest_buffer: Vec<Option<&str>>) -> Vec<String> {
        dest_buffer
            .into_iter()
            .map(|dest_opt| match dest_opt {
                Some(str) => str.to_owned(),          // Clone string slices
                None => String::from(Self::FALLBACK), // Allocate fallbacks
            })
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
    fn apply_map<'a>(map: &Map, src_row: &'a Row, dest_buffer: &mut Vec<Option<&'a str>>) {
        for (src, dest_opt) in map.iter().enumerate() {
            if let Some(dest) = *dest_opt {
                dest_buffer[dest] = Some(&src_row[src]);
            }
        }
    }
}

impl std::ops::Index<usize> for Row {
    type Output = String;

    fn index(&self, index: usize) -> &Self::Output {
        &self.row[index]
    }
}

/// An enum for handling the errors involved in the combination of a [`Row`] and a [`Map`].
#[derive(Debug, Copy, Clone, PartialEq)]
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

        for dest_opt in map_2.iter() {
            if let Some(dest) = dest_opt {
                if seen_indices.contains(dest) {
                    return Err(CombinationError::CollisionError { dest: *dest });
                }
            }
        }

        Ok(())
    }

    /// Checks if the length of a [`Row`] is at least as big as the length of the corresponding [`Map`].
    fn check_lengths(row: &Row, map: &Map) -> Result<(), CombinationError> {
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

impl std::error::Error for CombinationError {}
