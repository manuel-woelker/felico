use felico_base::bail;
use felico_base::result::FelicoResult;
use rand::random;
use std::marker::PhantomData;

pub struct TypedArena<T> {
    data: Vec<ArenaEntry<T>>,
    next_free: u32,
    cookie: u64,
}

#[allow(dead_code)]
enum ArenaEntry<T> {
    Occupied { value: T, generation: u8 },
    Free { next_free: u32, generation: u8 },
}

impl<T> Default for TypedArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TypedArenaHandle<T> {
    phantom_data: PhantomData<T>,
    key: u64,
}

impl<T> TypedArena<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            next_free: u32::MAX,
            cookie: (random::<u8>() as u64) << 56,
        }
    }

    pub fn add(&mut self, value: T) -> FelicoResult<TypedArenaHandle<T>> {
        let generation = 0;
        let key = (self.data.len() as u64 & 0xFFFFFFFF) | generation << 48 | self.cookie;
        self.data.push(ArenaEntry::Occupied {
            value,
            generation: 0,
        });
        Ok(TypedArenaHandle {
            phantom_data: PhantomData,
            key,
        })
    }

    pub fn get(&self, handle: &TypedArenaHandle<T>) -> FelicoResult<&T> {
        let (index_generation, index) = self.check_and_extract_index(handle)?;
        let entry = &self.data[index as usize];
        match entry {
            ArenaEntry::Occupied { value, generation } => {
                if index_generation != *generation {
                    bail!(
                        "Generation mismatch - expected: {}, actual: {}",
                        generation,
                        index_generation
                    );
                }
                Ok(value)
            }
            ArenaEntry::Free { .. } => bail!("Arena is free at index {}", index),
        }
    }

    fn check_and_extract_index(&self, index: &TypedArenaHandle<T>) -> FelicoResult<(u8, u64)> {
        let cookie = index.key & 0xFF00_0000_0000_0000;
        if cookie != self.cookie {
            bail!("Wrong cookie used to access arena");
        }
        let index_generation = ((index.key >> 48) & 0xFF) as u8;
        let index = index.key & 0xFFFFFFFF;
        if index >= self.data.len() as u64 {
            bail!("Index out of bounds: {} >= {}", index, self.data.len());
        };
        Ok((index_generation, index))
    }

    pub fn remove(&mut self, handle: &TypedArenaHandle<T>) -> FelicoResult<()> {
        let (index_generation, index) = self.check_and_extract_index(handle)?;
        let entry = &mut self.data[index as usize];
        match entry {
            ArenaEntry::Occupied { generation, .. } => {
                if index_generation != *generation {
                    bail!(
                        "Generation mismatch - expected: {}, actual: {}",
                        generation,
                        index_generation
                    );
                }
                *entry = ArenaEntry::Free {
                    next_free: self.next_free,
                    generation: index_generation + 1,
                };
                self.next_free = index as u32;
            }
            ArenaEntry::Free { .. } => bail!("Arena is free at index {}", index),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::typed_arena::TypedArena;
    use felico_base::result::FelicoResult;

    #[test]
    fn basic_add() -> FelicoResult<()> {
        let mut arena = TypedArena::new();
        let index = arena.add("foo")?;
        assert_eq!(arena.get(&index)?, &"foo");
        Ok(())
    }

    #[test]
    fn basic_remove() -> FelicoResult<()> {
        let mut arena = TypedArena::new();
        let index = arena.add("foo")?;
        arena.remove(&index)?;
        let error = arena.get(&index).expect_err("Expected error");
        assert_eq!(
            &error.to_test_string(),
            &"Error: Arena is free at index 0\n"
        );
        Ok(())
    }
}
