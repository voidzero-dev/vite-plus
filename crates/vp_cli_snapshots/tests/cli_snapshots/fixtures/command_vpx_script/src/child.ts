enum Source {
  Child = 'enum from child.ts',
}
process.send?.(Source.Child);
