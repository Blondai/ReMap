use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    map: Vec<Option<usize>>,
    max: usize,
}

impl Map {
    pub fn new(map: Vec<Option<usize>>) -> Result<Self, MapError> {
        let max: usize = Self::max_index(&map);

        MapError::check_map(&map)?;
        MapError::check_index(max)?;

        Ok(Self { map, max })
    }

    pub fn sequential(
        src_start: usize,
        src_end: usize,
        dest_start: usize,
    ) -> Result<Self, MapError> {
        MapError::check_range(src_start, src_end)?;
        MapError::check_overflow(src_start, src_end, dest_start)?;

        let max: usize = if src_start < src_end {
            dest_start + (src_end - src_start) - 1
        } else {
            0
        };

        let mut map: Vec<Option<usize>> = vec![None; src_end];

        let mut dest_index: usize = dest_start;

        // End is exclusive
        for src_index in src_start..src_end {
            map[src_index] = Some(dest_index);
            dest_index += 1;
        }

        Ok(Self { map, max })
    }

    pub fn splice(
        src_start_1: usize,
        src_end_1: usize,
        src_start_2: usize,
        src_end_2: usize,
    ) -> Result<(Self, Self), MapError> {
        let map_1: Map = Self::sequential(src_start_1, src_end_1, 0)?;
        let map_2: Map = Self::sequential(
            src_start_2,
            src_end_2,
            src_end_1.saturating_sub(src_start_1),
        )?;

        Ok((map_1, map_2))
    }

    pub fn identity(length: usize) -> Self {
        let map: Vec<Option<usize>> = (0..length).map(Some).collect();
        let max: usize = length.saturating_sub(1);

        Self { map, max }
    }

    pub fn as_slice(&self) -> &[Option<usize>] {
        &self.map
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn destination(&self, src: usize) -> Option<usize> {
        self.map.get(src).copied().flatten()
    }

    fn max_index(vec: &Vec<Option<usize>>) -> usize {
        vec.iter()
            .filter_map(|dest_opt| *dest_opt)
            .max()
            .unwrap_or(0)
    }

    pub fn max(&self) -> usize {
        self.max
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Option<usize>> {
        self.map.iter()
    }
}

impl std::ops::Index<usize> for Map {
    type Output = Option<usize>;

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

            // Adds comma
            if src < self.len().saturating_sub(1) {
                write!(format, ", ")?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MapError {
    DuplicateIndex { dest: usize },
    IndexTooLarge,
    RangeError { src_start: usize, src_end: usize },
}

impl MapError {
    fn check_map(map: &Vec<Option<usize>>) -> Result<(), MapError> {
        let mut seen_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for dest_opt in map {
            if let Some(dest) = *dest_opt {
                if !seen_indices.insert(dest) {
                    return Err(MapError::DuplicateIndex { dest });
                }
            }
        }

        Ok(())
    }

    fn check_index(max: usize) -> Result<(), MapError> {
        if max != usize::MAX {
            Ok(())
        } else {
            Err(MapError::IndexTooLarge)
        }
    }

    fn check_range(src_start: usize, src_end: usize) -> Result<(), MapError> {
        if src_start <= src_end {
            Ok(())
        } else {
            Err(MapError::RangeError { src_start, src_end })
        }
    }

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
            MapError::IndexTooLarge => write!(format, "Index ({}) is too large", usize::MAX),
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

impl std::error::Error for MapError {}
