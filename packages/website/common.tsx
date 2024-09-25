import * as fs from "node:fs/promises";
import prettier from "prettier";
import { JSX } from "preact";
import { render } from "preact-render-to-string";
import { indexHtml } from "./templates";

const renderHtml = async (element: JSX.Element) =>
  await prettier.format(`<!doctype html>\n${render(element)}`, {
    parser: "html",
  });

const out = "out";

const generate = async () => {
  for (const file of ["index.css"]) {
    await Bun.write(`${out}/${file}`, Bun.file(`src/${file}`));
  }
  await Bun.write(`${out}/index.html`, await renderHtml(indexHtml()));
};

export const build = async () => {
  await fs.rm(out, { recursive: true });
  await generate();
  const tmp = "tmp";
  const dist = "dist";
  try {
    await fs.rename(dist, tmp);
  } catch (_) {}
  await fs.rename(out, dist);
  await fs.rm(tmp, { recursive: true });
};
