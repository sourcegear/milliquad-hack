/*
 *  Copyright 2021 QuantumBadger
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

/// A vector with two u32 values.
pub type UVec2 = Vector2<u32>;

/// A vector containing two numeric values. This may represent a size or
/// position.
#[repr(C)]
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct Vector2<T>
{
    /// The horizontal component of the vector.
    pub x: T,
    /// The vertical component of the vector.
    pub y: T
}

impl<T> Vector2<T>
{
    /// Instantiates a new `Vector2` from the specified horizontal and vertical
    /// components.
    #[inline]
    #[must_use]
    pub const fn new(x: T, y: T) -> Self
    {
        Vector2 { x, y }
    }
}

