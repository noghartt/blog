import ReactMarkdown, { Components } from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { prism } from 'react-syntax-highlighter/dist/cjs/styles/prism';
import rehypeKatex from 'rehype-katex';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';

import type { Post } from '../../lib/posts';

import 'katex/dist/katex.min.css';
import { theme } from '../theme';

// TODO: I don't like the way that we componentize the things here and how I reproduce the text headers
//       in the future I want to go back here and refactor those things.
const defaultComponents: Components = {
  h1: ({ children }) => <h2>{children}</h2>,
  h2: ({ children }) => <h3>{children}</h3>,
  h3: ({ children }) => <h4>{children}</h4>,
  // TODO: Maybe it should be moved to another component, something like: components/typography/Paragraph.tsx
  p: ({ children, ...props }) => (
    <>
      <p {...props}>
        {children}
      </p>
      <style jsx>{`
        p {
          text-indent: 18px;
        }

        @media (max-width: ${theme.breakpoints.medium}) {
          text-align-justify: justify;
          text-indent: 10px;
        }
      `}</style>
    </>
  ),
  // TODO: This is also should be moved to a single component.
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
      );
  },
}

type Props = {
  post: Post;
  components?: Components;
}

export const Markdown = ({ components, post }: Props) => (
  <main>
    <h1>{post.metadata.title}</h1>
    <section>
      {post.metadata.tags.map(tag => (
        <p>#{tag}</p>
      ))}
    </section>
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

      h1 {
        font-size: 1.5rem;
        margin-bottom: 5px;
        text-align: justify;
      }

      section {
        display: flex;
        flex-wrap: wrap;
        gap: 5px;
        margin-top: 10px;
      }

      section > p {
        font-size: ${theme.fontSize.small};
        font-style: italic;
        color: rgba(0, 0, 0, 0.5);
        margin: 0;
        flex-shrink: 0;
      }

      @media (min-width: ${theme.breakpoints.large}) {
        h1 {
          font-size: ${theme.fontSize.large};
        }
      }
    `}</style>
  </main>
);
