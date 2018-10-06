// This file is a part of Eq.
// Copyright (C) 2018 Matthew Blount

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.

// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public
// License along with this program.  If not, see
// <https://www.gnu.org/licenses/.

use super::*;

// Get the next function to be executed.
fn fetch(
  code: &mut Vec<heap::Pointer>,
  heap: &mut heap::Heap) -> Result<heap::Pointer> {
  loop {
    let object = code.pop().ok_or(Error::Bug)?;
    if heap.is_sequence(object)? {
      let head = heap.get_sequence_head(object)?;
      let tail = heap.get_sequence_tail(object)?;
      code.push(tail);
      code.push(head);
    } else {
      return Ok(object);
    }
  }
}

// Return all of the code up to the next reset.
fn jump(
  code: &mut Vec<heap::Pointer>,
  heap: &mut heap::Heap) -> Result<heap::Pointer> {
  let mut buf = Vec::new();
  loop {
    let object = fetch(code, heap)?;
    if heap.is_reset(object)? {
      code.push(object);
      let mut xs = heap.new_id()?;
      for object in buf.iter().rev() {
        xs = heap.new_sequence(*object, xs)?;
      }
      return heap.new_block(xs);
    } else {
      buf.push(object);
    }
  }
}

// The current instruction and everything on the stack become dead code.
fn freeze(
  code: heap::Pointer,
  data: &mut Vec<heap::Pointer>,
  kill: &mut Vec<heap::Pointer>) {
  kill.append(data);
  kill.push(code);
}

/// Rewrite a term until it either reaches normal form or time runs
/// out.
pub fn reduce(
  root: heap::Pointer,
  heap: &mut heap::Heap,
  mut time: usize) -> Result<heap::Pointer> {
  let mut code = vec![root];
  let mut data = vec![];
  let mut kill = vec![];
  // The "kill" trick has some problems: if dead code later expands to
  // something containing a shift, then the meaning of the code
  // changes. I *think* it's okay if we only remove resets when
  // there's no dead code, but I'll need to test it more.
  while time > 0 && !code.is_empty() {
    time -= 1;
    let object = fetch(&mut code, heap)?;
    if heap.is_number(object)? {
      data.push(object);
    } else if heap.is_block(object)? {
      data.push(object);
    } else if heap.is_apply(object)? {
      // [A][B]a = B[A]
      if data.len() < 2 {
        freeze(object, &mut data, &mut kill);
        continue;
      }
      let func = data.pop().ok_or(Error::Underflow)?;
      let hide = data.pop().ok_or(Error::Underflow)?;
      assert(heap.is_block(func))?;
      let func_body = heap.get_block_body(func)?;
      code.push(hide);
      code.push(func_body);
    } else if heap.is_bind(object)? {
      // [A][B]b = [[A]B]
      if data.len() < 2 {
        freeze(object, &mut data, &mut kill);
        continue;
      }
      let func = data.pop().ok_or(Error::Underflow)?;
      let show = data.pop().ok_or(Error::Underflow)?;
      assert(heap.is_block(func))?;
      let func_body = heap.get_block_body(func)?;
      let sequence = heap.new_sequence(show, func_body)?;
      let block = heap.new_block(sequence)?;
      data.push(block);
    } else if heap.is_copy(object)? {
      // [A]c = [A] [A]
      if data.is_empty() {
        freeze(object, &mut data, &mut kill);
        continue;
      }
      let copy = data.last().ok_or(Error::Underflow)?;
      data.push(*copy);
    } else if heap.is_drop(object)? {
      // [A] d =
      if data.is_empty() {
        freeze(object, &mut data, &mut kill);
        continue;
      }
      data.pop().ok_or(Error::Underflow)?;
    } else if heap.is_fix(object)? {
      // [A]f = [[A]fA]
      if data.is_empty() {
        freeze(object, &mut data, &mut kill);
        continue;
      }
      let block = data.pop().ok_or(Error::Underflow)?;
      let block_body = heap.get_block_body(block)?;
      let lhs = heap.new_sequence(block, object)?;
      let rhs = heap.new_sequence(lhs, block_body)?;
      let fix = heap.new_block(rhs)?;
      data.push(fix);
    } else if heap.is_shift(object)? {
      // [A]sBr = [B]Ar
      // Is this correct? Should we crash instead?
      if data.is_empty() {
        freeze(object, &mut data, &mut kill);
        continue;
      }
      let callback = data.pop().ok_or(Error::Underflow)?;
      let callback_body = heap.get_block_body(callback)?;
      let continuation = jump(&mut code, heap)?;
      code.push(callback_body);
      data.push(continuation);
    } else if heap.is_reset(object)? {
      // r =
      // If there's dead code, we can't delete stuff.
      if !kill.is_empty() {
        freeze(object, &mut data, &mut kill);
      }
    } else if heap.is_id(object)? {
      //
    } else {
      freeze(object, &mut data, &mut kill);
    }
  }
  let mut xs = heap.new_id()?;
  for object in code.iter() {
    xs = heap.new_sequence(*object, xs)?;
  }
  for object in data.iter().rev() {
    xs = heap.new_sequence(*object, xs)?;
  }
  for object in kill.iter().rev() {
    xs = heap.new_sequence(*object, xs)?;
  }
  return Ok(xs);
}
