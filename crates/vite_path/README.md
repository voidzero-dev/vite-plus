# vite_path

Provides path typed with its relativity: `AbsolutePath(Buf)` and `RelativePath(Buf)`, and safe methods to convert between them (for example, `AbsolutePath::join(RelativePath)` produces `AbsolutePathBuf`).
