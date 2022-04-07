import path from 'path';
import fs from 'fs/promises';
import matter from 'gray-matter';
import slugify from 'slugify';

const postsDirectory = path.join(process.cwd(), 'posts');

// This type should match the value from the frontmatter YAML
// in `<rootDir>/posts/**/README.md` files and some another values
// that came when matter parse the markdown file.
export type PostMetadata = {
  id: string;
  title: string;
  date: string;
  slug: string;
  tags: string[];
  devto?: string;
  draft?: boolean;
}

export type Post = {
  content: string;
  metadata: PostMetadata;
};

export type Posts = Post[];

const listPostsOnFolder = async () => {
  // TODO: Maybe it should be turn into an asynchronous function using the fs/promises module.
  const fsSync = await import('fs');

  const postFolders = await fs.readdir(postsDirectory);
  const posts = postFolders.map(postFolder => path.join(postsDirectory, postFolder));

  const mds = posts.map(post => {
    const postPath = path.join(post, 'README.md');

    const md = fsSync.readFileSync(postPath, { encoding: 'utf8' });

    return md;
  });

  return mds;
}

const parseWithMatter = (rawString: string[]): Posts =>
  rawString
    .map(raw => matter(raw))
    .map(({ content, data }) => ({ content, metadata: data })) as Posts;

export const getAllPosts = async (): Promise<Posts> => {
  const rawPosts = await listPostsOnFolder();

  const matteredPosts = parseWithMatter(rawPosts);

  return matteredPosts;
};

// TODO: I don't like the way that we handle this.
//       In the future, improve this function, PLEASE!
export const getPostBySlug = async (slug?: string) => {
  const posts = await getAllPosts();

  const foundPost = posts.find(post => post.metadata.slug === slug);

  return foundPost;
}
