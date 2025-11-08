use crate::Map;

use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    row: Vec<String>,
}

impl Row {
    const FALLBACK: &'static str = "";

    pub fn new(row: Vec<String>) -> Self {
        Self { row }
    }

    pub fn as_slice(&self) -> &[String] {
        &self.row
    }

    pub fn len(&self) -> usize {
        self.row.len()
    }

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
        let mut dest_data: Vec<Option<&str>> = vec![None; dest_len];

        Self::apply_map(map_1, row_1, &mut dest_data);
        Self::apply_map(map_2, row_2, &mut dest_data);

        let row: Vec<String> = dest_data
            .into_iter()
            .map(|dest_opt| String::from(dest_opt.unwrap_or(Self::FALLBACK)))
            .collect();

        Ok(Row::new(row))
    }

    pub fn reorder(&self, map: &Map) -> Result<Self, CombinationError> {
        CombinationError::check_lengths(self, map)?;

        let dest_len: usize = map.max() + 1;

        let mut dest_data: Vec<Option<&str>> = vec![None; dest_len];

        Self::apply_map(map, self, &mut dest_data);

        let row: Vec<String> = dest_data
            .into_iter()
            .map(|opt| String::from(opt.unwrap_or(Self::FALLBACK)))
            .collect();

        Ok(Row::new(row))
    }

    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.row.iter()
    }

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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CombinationError {
    CollisionError { dest: usize },
    LengthError { map_len: usize, row_len: usize },
}

impl CombinationError {
    fn check_collision(map_1: &Map, map_2: &Map) -> Result<(), CombinationError> {
        // map_1 and map_2 per definitionem only contain each index once.
        let seen_indices: std::collections::HashSet<usize> =
            map_1.iter().filter_map(|dest_opt| *dest_opt).collect();

        for dest_opt in map_2.iter() {
            if let Some(dest) = dest_opt {
                if seen_indices.contains(dest) {
                    return Err(CombinationError::CollisionError { dest: *dest });
                }
            }
        }

        Ok(())
    }

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
            Self::LengthError {
                row_len: csv_len,
                map_len,
            } => write!(
                format,
                "Map of length {} is longer than row of length {}",
                map_len, csv_len
            ),
        }
    }
}

impl std::error::Error for CombinationError {}
