import path from 'path';
import fs from 'fs';

import { slugify } from './slugify';

const run = async () => {
  const dirRoot = path.resolve(process.cwd());

  const template = `\
---
slug: {{slug}}
title: {{title}}
createdAt: {{createdAt}}
---
`;

  const [_, __, title, ...tags] = process.argv;

  let frontmatter = template
    .split('\n')
    .filter(Boolean)
    .filter(str => str !== '---').join('\n');

  const createdAt = new Date();

  const slug = slugify(title);

  frontmatter = frontmatter.replace('{{slug}}', `"${slug}"`)
  frontmatter = frontmatter.replace('{{title}}', `"${title}"`);
  frontmatter = frontmatter.replace('{{createdAt}}', `${createdAt.toISOString()}`);

  if (Array.isArray(tags) && tags.length > 0) {
    const tagsFormatted = tags
      .map(tag => slugify(tag))
      .map((tag, i) => `  - ${tag}`)
      .join('\n');

    frontmatter += `\ntags:\n${tagsFormatted}`
  }

  const data = `\
---
${frontmatter}
---
`;

  const dirBlog = path.resolve(dirRoot, 'src', 'content', 'til');

  fs.writeFileSync(path.resolve(dirBlog, `${createdAt.toISOString()}-${slug}.md`), data, { encoding: 'utf-8' });
}

run();
