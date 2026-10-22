---
name: go-stream-io
subject: Buffering, streaming, and deadlines for Go I/O.
requirement: >-
  Stream large or unbounded I/O with io.Copy, bufio.Reader, or Scanner instead
  of io.ReadAll; wrap many small reads with bufio; check Scanner.Err(); and
  bound the token or body size when input is untrusted.
applicability: |-
  Exception: Small bounded payloads where the full bytes are required, local
  files with known size, and already-buffered readers.
paths:
  - "*.go"
purpose: Whole-input reads scale memory with input, and unbuffered small reads add syscall overhead.
---
```diff
-data, _ := io.ReadAll(resp.Body)
+defer resp.Body.Close()
+if _, err := io.Copy(os.Stdout, resp.Body); err != nil {
+    return fmt.Errorf("copy response: %w", err)
+}
```

Sources: [bufio](https://pkg.go.dev/bufio), [io.Copy](https://pkg.go.dev/io#Copy), [io.LimitReader](https://pkg.go.dev/io#LimitReader).
