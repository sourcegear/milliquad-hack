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

/// Measures the amount of time elapsed since its creation.
pub struct Stopwatch
{
    start: f64
}

impl Stopwatch
{
    /// Creates a new Stopwatch, starting at the current time.
    #[inline]
    pub fn new() -> Result<Self, i32> // TODO just return Self, no Result
    {
        let start = miniquad::date::now();

        Ok(Self { start })
    }

    /// Returns the number of seconds since the Stopwatch was created.
    #[inline]
    pub fn secs_elapsed(&self) -> f64
    {
        miniquad::date::now() - self.start
    }
}

