// This file is a part of Sundial.
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

/// An error that might occur during computation.
#[derive(Debug, Copy, Clone)]
pub enum Error {
  Time,
  Space,
  Tag,
  Stub,
  Bug,
  Null,
  Assert,
  Syntax,
  Underflow,
  Home,
}

/// The result of a computation.
pub type Result<T> = std::result::Result<T, Error>;

/// A Sundial opcode.
#[derive(Debug, Copy, Clone)]
enum Opcode {
  App,
  Box,
  Cat,
  Copy,
  Drop,
  Swap,
  Prop,
  Forall,
}

/// Halt the computation if the given condition is false.
fn assert(flag: Result<bool>) -> Result<()> {
  match flag {
    Ok(true) => {
      return Ok(());
    }
    Ok(false) => {
      return Err(Error::Assert);
    }
    Err(error) => {
      return Err(error);
    }
  }
}

pub const WORD_PATTERN: &'static str = r"[a-z0-9-]+";

lazy_static! {
  static ref WORD_REGEX: regex::Regex = {
    regex::Regex::new(WORD_PATTERN).unwrap()
  };
  static ref POD_INSERT_REGEX: regex::Regex = {
    let src = format!(r"^:({})\s+(.*)", WORD_PATTERN);
    regex::Regex::new(&src).unwrap()
  };
  static ref POD_DELETE_REGEX: regex::Regex = {
    let src = format!(r"^~({})\s*", WORD_PATTERN);
    regex::Regex::new(&src).unwrap()
  };
  static ref HINT_REGEX: regex::Regex = {
    let src = format!(r"^\(({})\)$", WORD_PATTERN);
    regex::Regex::new(&src).unwrap()
  };
}

/// A pointer to some object.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Gc {
  index: usize,
  generation: u64,
}

use std::rc::Rc;
use std::collections::HashMap;

type Library = HashMap<Rc<str>, Gc>;

enum Object {
  Id,
  Opcode(Opcode),
  Word(Rc<str>),
  Hint(Rc<str>),
  Block(Gc),
  Sequence(Gc, Gc),
}

struct Node {
  object: Object,
  generation: u64,
  is_visible: bool,
}

/// A garbage-collected heap.
struct Heap {
  nodes: Vec<Option<Node>>,
  generation: u64,
}

impl Gc {
  fn new(index: usize, generation: u64) -> Self {
    Gc {
      index: index,
      generation: generation,
    }
  }
}

impl Object {
  fn is_id(&self) -> bool {
    match self {
      Object::Id => true,
      _ => false,
    }
  }

  fn is_opcode(&self) -> bool {
    match self {
      Object::Opcode(_) => true,
      _ => false,
    }
  }

  fn is_word(&self) -> bool {
    match self {
      Object::Word(_) => true,
      _ => false,
    }
  }

  fn is_hint(&self) -> bool {
    match self {
      Object::Hint(_) => true,
      _ => false,
    }
  }

  fn is_block(&self) -> bool {
    match self {
      Object::Block(_) => true,
      _ => false,
    }
  }

  fn is_sequence(&self) -> bool {
    match self {
      Object::Sequence(_, _) => true,
      _ => false,
    }
  }
}

impl Node {
  fn new(object: Object, generation: u64) -> Self {
    Node {
      object: object,
      generation: generation,
      is_visible: false,
    }
  }
}

impl Heap {
  /// Creates a heap with the given capacity.
  fn with_capacity(capacity: usize) -> Self {
    let mut nodes = Vec::with_capacity(capacity);
    for _ in 0..capacity {
      nodes.push(None);
    }
    Heap {
      nodes: nodes,
      generation: 0,
    }
  }

  fn new_id(&mut self) -> Result<Gc> {
    let object = Object::Id;
    return self.put(object);
  }

  fn new_opcode(&mut self, opcode: Opcode) -> Result<Gc> {
    let object = Object::Opcode(opcode);
    return self.put(object);
  }

  fn new_word(&mut self, value: Rc<str>) -> Result<Gc> {
    let object = Object::Word(value);
    return self.put(object);
  }

  fn new_hint(&mut self, value: Rc<str>) -> Result<Gc> {
    let object = Object::Hint(value);
    return self.put(object);
  }

  fn new_block(&mut self, body: Gc) -> Result<Gc> {
    let object = Object::Block(body);
    return self.put(object);
  }

  fn new_sequence(&mut self, fst: Gc, snd: Gc) -> Result<Gc> {
    if self.is_id(fst)? {
      return Ok(snd);
    }
    if self.is_sequence(fst)? {
      let fst_fst = self.get_sequence_fst(fst)?;
      let fst_snd = self.get_sequence_snd(fst)?;
      let rhs = self.new_sequence(fst_snd, snd)?;
      return self.new_sequence(fst_fst, rhs);
    }
    let object = Object::Sequence(fst, snd);
    return self.put(object);
  }

  fn is_id(&self, pointer: Gc) -> Result<bool> {
    let object = self.get_ref(pointer)?;
    return Ok(object.is_id());
  }

  fn is_opcode(&self, pointer: Gc) -> Result<bool> {
    let object = self.get_ref(pointer)?;
    return Ok(object.is_opcode());
  }

  fn is_word(&self, pointer: Gc) -> Result<bool> {
    let object = self.get_ref(pointer)?;
    return Ok(object.is_word());
  }

  fn is_hint(&self, pointer: Gc) -> Result<bool> {
    let object = self.get_ref(pointer)?;
    return Ok(object.is_hint());
  }

  fn is_block(&self, pointer: Gc) -> Result<bool> {
    let object = self.get_ref(pointer)?;
    return Ok(object.is_block());
  }

  fn is_sequence(&self, pointer: Gc) -> Result<bool> {
    let object = self.get_ref(pointer)?;
    return Ok(object.is_sequence());
  }

  fn get_opcode(&self, pointer: Gc) -> Result<Opcode> {
    match self.get_ref(pointer)? {
      &Object::Opcode(ref value) => {
        return Ok(*value);
      }
      _ => {
        return Err(Error::Tag);
      }
    }
  }

  fn get_word(&self, pointer: Gc) -> Result<Rc<str>> {
    match self.get_ref(pointer)? {
      &Object::Word(ref value) => {
        return Ok(value.clone());
      }
      _ => {
        return Err(Error::Tag);
      }
    }
  }

  fn get_hint(&self, pointer: Gc) -> Result<Rc<str>> {
    match self.get_ref(pointer)? {
      &Object::Hint(ref value) => {
        return Ok(value.clone());
      }
      _ => {
        return Err(Error::Tag);
      }
    }
  }

  fn get_block_body(&self, pointer: Gc) -> Result<Gc> {
    match self.get_ref(pointer)? {
      &Object::Block(ref body) => {
        return Ok(*body);
      }
      _ => {
        return Err(Error::Tag);
      }
    }
  }

  fn get_sequence_fst(&self, pointer: Gc) -> Result<Gc> {
    match self.get_ref(pointer)? {
      &Object::Sequence(ref fst, _) => {
        return Ok(*fst);
      }
      _ => {
        return Err(Error::Tag);
      }
    }
  }

  fn get_sequence_snd(&self, pointer: Gc) -> Result<Gc> {
    match self.get_ref(pointer)? {
      &Object::Sequence(_, ref snd) => {
        return Ok(*snd);
      }
      _ => {
        return Err(Error::Tag);
      }
    }
  }

  fn mark(&mut self, root: Gc) -> Result<()> {
    match &mut self.nodes[root.index] {
      &mut Some(ref mut node) => {
        if node.generation != root.generation {
          return Err(Error::Null);
        }
        node.is_visible = true;
        match &node.object {
          &Object::Block(body) => {
            return self.mark(body);
          }
          &Object::Sequence(fst, snd) => {
            self.mark(fst)?;
            return self.mark(snd);
          }
          _ => {
            return Ok(());
          }
        }
      }
      _ => {
        return Err(Error::Null);
      }
    }
  }

  fn sweep(&mut self) -> Result<()> {
    let mut nodes_deleted = 0;
    for maybe_node in self.nodes.iter_mut() {
      let should_delete_node;
      if let Some(ref mut node) = maybe_node {
        if node.is_visible {
          node.is_visible = false;
          should_delete_node = false;
        } else {
          should_delete_node = true;
        }
      } else {
        should_delete_node = false;
      }
      if should_delete_node {
        *maybe_node = None;
        nodes_deleted += 1;
      }
    }
    self.generation += 1;
    println!(
      "[gc] deleted: {} generation: {}", nodes_deleted, self.generation);
    return Ok(());
  }

  fn put(&mut self, object: Object) -> Result<Gc> {
    for (index, maybe_node) in self.nodes.iter_mut().enumerate() {
      if maybe_node.is_some() {
        continue;
      }
      let node = Node::new(object, self.generation);
      let pointer = Gc::new(index, self.generation);
      *maybe_node = Some(node);
      return Ok(pointer);
    }
    return Err(Error::Space);
  }

  fn get_ref(&self, pointer: Gc) -> Result<&Object> {
    match &self.nodes[pointer.index] {
      &Some(ref node) => {
        if node.generation == pointer.generation {
          return Ok(&node.object);
        }
        return Err(Error::Null);
      }
      None => {
        return Err(Error::Null);
      }
    }
  }
}

fn parse(src: &str, heap: &mut Heap) -> Result<Gc> {
  let mut build = Vec::new();
  let mut stack = Vec::new();
  let src = src.replace("[", "[ ");
  let src = src.replace("]", " ]");
  for word in src.split_whitespace() {
    match word {
      "[" => {
        stack.push(build);
        build = Vec::new();
      }
      "]" => {
        let prev = stack.pop().ok_or(Error::Syntax)?;
        let mut xs = heap.new_id()?;
        for object in build.iter().rev() {
          xs = heap.new_sequence(*object, xs)?;
        }
        xs = heap.new_block(xs)?;
        build = prev;
        build.push(xs);
      }
      "a" => {
        let opcode = Opcode::App;
        let object = heap.new_opcode(opcode)?;
        build.push(object);
      }
      "b" => {
        let opcode = Opcode::Box;
        let object = heap.new_opcode(opcode)?;
        build.push(object);
      }
      "c" => {
        let opcode = Opcode::Cat;
        let object = heap.new_opcode(opcode)?;
        build.push(object);
      }
      "d" => {
        let opcode = Opcode::Copy;
        let object = heap.new_opcode(opcode)?;
        build.push(object);
      }
      "e" => {
        let opcode = Opcode::Drop;
        let object = heap.new_opcode(opcode)?;
        build.push(object);
      }
      "f" => {
        let opcode = Opcode::Swap;
        let object = heap.new_opcode(opcode)?;
        build.push(object);
      }
      "g" => {
        let opcode = Opcode::Forall;
        let object = heap.new_opcode(opcode)?;
        build.push(object);
      }
      "h" => {
        let opcode = Opcode::Prop;
        let object = heap.new_opcode(opcode)?;
        build.push(object);
      }
      _ => {
        if word.len() == 1 {
          if word.chars().all(|x| x.is_lowercase()) {
            return Err(Error::Syntax);
          }
        }
        if let Some(data) = HINT_REGEX.captures(&word) {
          let name = data.get(1).ok_or(Error::Bug)?.as_str();
          let object = heap.new_hint(name.into())?;
          build.push(object);
        } else {
          let object = heap.new_word(word.into())?;
          build.push(object);
        }
      }
    }
  }
  if !stack.is_empty() {
    return Err(Error::Syntax);
  }
  let mut xs = heap.new_id()?;
  for object in build.iter().rev() {
    xs = heap.new_sequence(*object, xs)?;
  }
  return Ok(xs);
}

fn quote(root: Gc, heap: &Heap, buf: &mut String) -> Result<()> {
  match heap.get_ref(root)? {
    &Object::Id => {
      //
    }
    &Object::Opcode(ref value) => {
      match value {
        Opcode::App => {
          buf.push('a');
        }
        Opcode::Box => {
          buf.push('b');
        }
        Opcode::Cat => {
          buf.push('c');
        }
        Opcode::Copy => {
          buf.push('d');
        }
        Opcode::Drop => {
          buf.push('e');
        }
        Opcode::Swap => {
          buf.push('f');
        }
        Opcode::Forall => {
          buf.push('g');
        }
        Opcode::Prop => {
          buf.push('h');
        }
      }
    }
    &Object::Word(ref value) => {
      buf.push_str(&value);
    }
    &Object::Hint(ref value) => {
      buf.push('(');
      buf.push_str(&value);
      buf.push(')');
    }
    &Object::Block(body) => {
      buf.push('[');
      quote(body, heap, buf)?;
      buf.push(']');
    }
    &Object::Sequence(fst, snd) => {
      quote(fst, heap, buf)?;
      if !heap.is_id(snd)? {
        buf.push(' ');
        quote(snd, heap, buf)?;
      }
    }
  }
  return Ok(());
}

fn reduce(
  continuation: Gc,
  heap: &mut Heap,
  tab: &Library,
  mut time_quota: u64) -> Result<Gc> {
  let mut thread = Thread::with_continuation(continuation);
  while time_quota > 0 && thread.has_continuation() {
    time_quota -= 1;
    thread.step(heap, tab)?;
  }
  if thread.has_continuation() {
    let snd = thread.get_continuation(heap)?;
    let fst = thread.get_environment(heap)?;
    return heap.new_sequence(fst, snd);
  }
  return thread.get_environment(heap);
}

use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct Frame {
  con: VecDeque<Gc>,
  env: Vec<Gc>,
  err: Vec<Gc>,
}

impl Frame {
  fn new(root: Gc) -> Self {
    let mut con = VecDeque::new();
    con.push_back(root);
    Frame {
      con: con,
      env: vec![],
      err: vec![],
    }
  }
}

struct Thread {
  frame: Frame,
}

impl Thread {
  fn with_continuation(continuation: Gc) -> Self {
    Thread {
      frame: Frame::new(continuation),
    }
  }

  fn has_continuation(&self) -> bool {
    return !self.frame.con.is_empty();
  }

  fn get_continuation(
    &mut self, heap: &mut Heap) -> Result<Gc> {
    let mut xs = heap.new_id()?;
    for object in self.frame.con.iter() {
      xs = heap.new_sequence(*object, xs)?;
    }
    self.frame.con.clear();
    return Ok(xs);
  }

  fn push_continuation_front(&mut self, data: Gc) {
    self.frame.con.push_front(data);
  }

  fn push_continuation_back(&mut self, data: Gc) {
    self.frame.con.push_back(data);
  }

  fn pop_continuation(
    &mut self, heap: &mut Heap) -> Result<Gc> {
    loop {
      let code = self.frame.con.pop_front().ok_or(Error::Bug)?;
      if heap.is_sequence(code)? {
        let fst = heap.get_sequence_fst(code)?;
        let snd = heap.get_sequence_snd(code)?;
        self.frame.con.push_front(snd);
        self.frame.con.push_front(fst);
      } else {
        return Ok(code);
      }
    }
  }

  fn is_monadic(&self) -> bool {
    return self.frame.env.len() >= 1;
  }

  fn is_dyadic(&self) -> bool {
    return self.frame.env.len() >= 2;
  }

  fn get_environment(
    &mut self, heap: &mut Heap) -> Result<Gc> {
    let mut xs = heap.new_id()?;
    for object in self.frame.env.iter().rev() {
      xs = heap.new_sequence(*object, xs)?;
    }
    for object in self.frame.err.iter().rev() {
      xs = heap.new_sequence(*object, xs)?;
    }
    self.frame.env.clear();
    self.frame.err.clear();
    return Ok(xs);
  }

  fn push_environment(&mut self, data: Gc) {
    self.frame.env.push(data);
  }

  fn pop_environment(&mut self) -> Result<Gc> {
    return self.frame.env.pop().ok_or(Error::Underflow);
  }

  fn peek_environment(&mut self) -> Result<Gc> {
    return self.frame.env.last().map(|x| *x).ok_or(Error::Underflow);
  }

  fn thunk(&mut self, root: Gc) {
    self.frame.err.append(&mut self.frame.env);
    self.frame.err.push(root);
  }

  fn step(
    &mut self,
    heap: &mut Heap,
    tab: &HashMap<Rc<str>, Gc>) -> Result<()> {
    let code = self.pop_continuation(heap)?;
    if heap.is_block(code)? {
      self.push_environment(code);
    } else if heap.is_opcode(code)? {
      match heap.get_opcode(code)? {
        Opcode::App => {
          if !self.is_monadic() {
            self.thunk(code);
            return Ok(());
          }
          let source = self.pop_environment()?;
          let target = heap.get_block_body(source)?;
          self.push_continuation_front(target);
        }
        Opcode::Box => {
          if !self.is_monadic() {
            self.thunk(code);
            return Ok(());
          }
          let source = self.pop_environment()?;
          let target = heap.new_block(source)?;
          self.push_environment(target);
        }
        Opcode::Cat => {
          if !self.is_dyadic() {
            self.thunk(code);
            return Ok(());
          }
          let rhs = self.pop_environment()?;
          let lhs = self.pop_environment()?;
          let rhs_body = heap.get_block_body(rhs)?;
          let lhs_body = heap.get_block_body(lhs)?;
          let target_body = heap.new_sequence(lhs_body, rhs_body)?;
          let target = heap.new_block(target_body)?;
          self.push_environment(target);
        }
        Opcode::Copy => {
          if !self.is_monadic() {
            self.thunk(code);
            return Ok(());
          }
          let source = self.peek_environment()?;
          self.push_environment(source);
        }
        Opcode::Drop => {
          if !self.is_monadic() {
            self.thunk(code);
            return Ok(());
          }
          self.pop_environment()?;
        }
        Opcode::Swap => {
          if !self.is_dyadic() {
            self.thunk(code);
            return Ok(());
          }
          let fst = self.pop_environment()?;
          let snd = self.pop_environment()?;
          self.push_environment(fst);
          self.push_environment(snd);
        }
        Opcode::Prop | Opcode::Forall => {
          self.thunk(code);
          return Ok(());
        }
      }
    } else if heap.is_word(code)? {
      let code_value = heap.get_word(code)?;
      match tab.get(&code_value) {
        Some(binding) => {
          self.push_continuation_front(*binding);
        }
        None => {
          self.thunk(code);
        }
      }
      return Ok(());
    } else if heap.is_id(code)? || heap.is_hint(code)? {
      return Ok(());
    } else {
      return Err(Error::Bug);
    }
    return Ok(());
  }
}

pub struct Pod {
  heap: Heap,
  tab: Library,
}

impl Pod {
  fn with_heap(heap: Heap) -> Self {
    Pod {
      heap: heap,
      tab: HashMap::new(),
    }
  }

  pub fn from_string(
    src: &str,
    space_quota: usize,
    time_quota: u64) -> Result<Self> {
    let heap = Heap::with_capacity(space_quota);
    let mut pod = Pod::with_heap(heap);
    for line in src.lines() {
      pod.eval(line, time_quota)?;
    }
    return Ok(pod);
  }

  pub fn default(space_quota: usize, time_quota: u64) -> Result<Self> {
    let home = std::env::var("SUNDIAL_HOME").or(Err(Error::Home))?;
    let path: std::path::PathBuf = [&home, "pod", "default.md"].iter().collect();
    let src = std::fs::read_to_string(path).or(Err(Error::Home))?;
    return Pod::from_string(&src, space_quota, time_quota);
  }

  pub fn eval(&mut self, src: &str, time_quota: u64) -> Result<String> {
    let mut dst = String::new();
    if let Some(data) = POD_INSERT_REGEX.captures(src) {
      let key: Rc<str> = data.get(1).expect("key").as_str().into();
      let value_src = data.get(2).expect("value").as_str();
      let value = parse(value_src, &mut self.heap)?;
      let value = reduce(
        value, &mut self.heap, &self.tab, time_quota)?;
      self.tab.insert(key.clone(), value);
      dst.push(':');
      dst.push_str(&key);
      dst.push(' ');
      quote(value, &mut self.heap, &mut dst)?;
    } else if let Some(data) = POD_DELETE_REGEX.captures(src) {
      let key: Rc<str> = data.get(1).expect("key").as_str().into();
      self.tab.remove(&key);
      dst.push('~');
      dst.push_str(&key);
    } else {
      let source = parse(src, &mut self.heap)?;
      let target = reduce(
        source, &mut self.heap, &self.tab, time_quota)?;
      quote(target, &mut self.heap, &mut dst)?;
    }
    for pointer in self.tab.values() {
      self.heap.mark(*pointer)?;
    }
    self.heap.sweep()?;
    return Ok(dst);
  }

  pub fn to_string(&self) -> Result<String> {
    let mut target = String::new();
    let mut keys: Vec<Rc<str>> = self.tab.keys()
      .map(|x| x.clone()).collect();
    keys.sort();
    for key in keys.iter() {
      let value = self.tab.get(key).unwrap();
      target.push(':');
      target.push_str(&key);
      target.push(' ');
      quote(*value, &self.heap, &mut target)?;
      target.push('\n');
    }
    return Ok(target);
  }
}

#[test]
fn primitives() {
  let space   = 1024;
  let time    = 1024;
  let mut pod = Pod::from_string("", space, time).unwrap();
  let mut check = |source, expected| {
    println!("{} => {}", source, expected);
    let target = pod.eval(source, time).unwrap();
    assert_eq!(expected, &target);
  };
  check("", "");
  check("[A]", "[A]");
  check("[[A]]", "[[A]]");
  check("[A] [B]", "[A] [B]");
  check("a", "a");
  check("b", "b");
  check("c", "c");
  check("d", "d");
  check("e", "e");
  check("f", "f");
  check("g", "g");
  check("h", "h");
  check("[A] a", "A");
  check("[A] b", "[[A]]");
  check("[A] [B] c", "[A B]");
  check("[A] c", "[A] c");
  check("[A] d", "[A] [A]");
  check("[A] e", "");
  check("[A] [B] f", "[B] [A]");
  check("[A] f", "[A] f");
  check("[A] [B] b c", "[A [B]]");
  check("[A] g", "[A] g");
  check("[A] [B] g", "[A] [B] g");
  check("[A] h", "[A] h");
}
