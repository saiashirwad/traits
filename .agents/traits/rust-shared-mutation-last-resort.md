---
name: rust-shared-mutation-last-resort
subject: Rc, RefCell, Arc, Mutex, RwLock, and Box used as default containers.
requirement: >-
  Do not start with Rc, RefCell, Arc, Mutex, or RwLock. Use plain owned fields
  and & or &mut parameters. Introduce Rc and RefCell only for genuine
  single-thread shared ownership or interior mutability, and Arc, Mutex, or
  RwLock only when more than one owner or a thread or task boundary is visible.
  Prefer one lock over several, message passing or narrower locks, and RwLock
  for read-mostly data.
applicability: |-
  Exception: Graphs with parent and child links, caches, callback registries,
  APIs that require interior mutability, state genuinely shared by request
  handlers or worker tasks, and Arc handles cloned cheaply by design.
paths:
  - "*.rs"
purpose: Prevent designs that turn compile-time borrow errors into runtime deadlocks, panics, and refcount churn.
---
```diff
-struct App { users: Arc<Mutex<Vec<User>>> }
+struct App { users: Vec<User> }
+
+impl App {
+    fn add(&mut self, user: User) { self.users.push(user); }
+}
```

Sources: [Effective Rust: Borrows](https://www.lurklurk.org/effective-rust/borrows.html), [Effective Rust: Deadlocks](https://www.lurklurk.org/effective-rust/deadlock.html), [Clippy: arc_with_non_send_sync](https://rust-lang.github.io/rust-clippy/master/index.html#arc_with_non_send_sync).
