import markdownit from "markdown-it";
import * as fs from "node:fs/promises";
import { JSX } from "preact";
import { render } from "preact-render-to-string";
import prettier from "prettier";
import { indexHtml } from "./templates";

const renderHtml = async (element: JSX.Element) =>
  await prettier.format(`<!doctype html>\n${render(element)}`, {
    parser: "html",
  });

const out = "out";

const generate = async () => {
  const md = markdownit();

  for (const file of ["index.css"]) {
    await Bun.write(`${out}/${file}`, Bun.file(`src/${file}`));
  }
  await Bun.write(
    `${out}/index.html`,
    await renderHtml(
      indexHtml({
        body: (
          <div
            dangerouslySetInnerHTML={{
              __html: md.render(await Bun.file("src/index.md").text()),
            }}
          ></div>
        ),
      }),
    ),
  );
};

export const build = async () => {
  await fs.rm(out, { force: true, recursive: true });
  await generate();
  const tmp = "tmp";
  const dist = "dist";
  try {
    await fs.rename(dist, tmp);
  } catch (_) {}
  await fs.rename(out, dist);
  await fs.rm(tmp, { force: true, recursive: true });
};
