import { JSX } from "preact";

export const indexHtml = ({ body }: { body: JSX.Element }) => (
  <html lang="en-us">
    <head>
      <meta charset="utf-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1" />
      <link rel="stylesheet" href="/index.css" />
      <title>Adroit</title>
    </head>
    <body>{body}</body>
  </html>
);
