import ReactMarkdown, { Components } from 'react-markdown';
import rehypeKatex from 'rehype-katex';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';

import type { Post } from '../../lib/posts';

import 'katex/dist/katex.min.css';
import { theme } from '../theme';

const defaultComponents: Components = {
  h1: ({ children, level }) => (
    <h2>{children}</h2>
  ),
}

type Props = {
  post: Post;
  components?: Components;
}

export const Markdown = ({ components, post }: Props) => (
  <main>
    <h1>{post.metadata.title}</h1>
    <ReactMarkdown
      components={{ ...defaultComponents, ...components }}
      remarkPlugins={[remarkMath, remarkGfm]}
      rehypePlugins={[rehypeKatex]}
    >
      {post.content}
    </ReactMarkdown>
    <style jsx>{`
      main {
        width: 100%;
      }

      p {
        font-size: ${theme.fontSize.large};
      }
    `}</style>
  </main>
);
