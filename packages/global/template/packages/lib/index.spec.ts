import { test, expect } from "vitest";
import { getOne } from "./index.ts";

test("test", () => {
  expect(getOne()).toBe(1);
});
