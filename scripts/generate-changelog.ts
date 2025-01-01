import fs from 'fs/promises';
import path from 'path';

const cwd = process.cwd();

(async () => {
  const [, , title] = process.argv;

  console.log(title);

  const changelogPath = path.join(cwd, 'src', 'content', 'changelog', 'changelog.json');
  const changelog = await fs.readFile(changelogPath, 'utf-8');
  const changelogParse = JSON.parse(changelog);

  const changelogUpdated = [
    ...changelogParse,
    {
      time: new Date().toISOString(),
      title,
    },
  ];

  await fs.writeFile(changelogPath, JSON.stringify(changelogUpdated));
})();
