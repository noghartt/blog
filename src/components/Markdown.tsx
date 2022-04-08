import ReactMarkdown, { Components } from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { prism } from 'react-syntax-highlighter/dist/cjs/styles/prism';
import rehypeKatex from 'rehype-katex';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';

import type { Post } from '../../lib/posts';

import 'katex/dist/katex.min.css';
import { theme } from '../theme';

const defaultComponents: Components = {
  h1: ({ children }) => <h2>{children}</h2>,
  h2: ({ children }) => <h3>{children}</h3>,
  h3: ({ children }) => <h4>{children}</h4>,
  code: ({ node, inline, className, children, lang, ...props }) => {
    const match = /language-(\w+)/.exec(className || '');

    return !inline && match ?
      (
        <SyntaxHighlighter
          style={prism}
          language={match[1]}

        >
          {String(children).replace(/\n$/, '')}
        </SyntaxHighlighter>
      ) : (
        <code {...{ className, ...props }}>
          {children}
        </code>
      )
  },
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
