import Link from "next/link";
import { LinkProps } from "next/link";

import { theme } from "../theme";

export type ArticleProps = LinkProps & {
  title: string;
  date: string;
};

export const Article = ({ title, date, ...linkProps }: ArticleProps) => (
  <section>
    <p className="article-date">{date}</p>
    <Link {...linkProps}>
      <a className="article-title">{title}</a>
    </Link>
    <style jsx>{`
        section {
          display: flex;
          width: 100%;
          gap: 20px;
        }

        p {
          margin: 0;
        }

        .article-date {
          font-style: italic;
          font-size: ${theme.fontSize.base}
        }

        .article-title {
          font-size: ${theme.fontSize.base};
          font-family: ${theme.fontFamily.monospace};
        }

        @media (max-width: ${theme.breakpoints.medium}) {
          section {
            flex-direction: column-reverse;
            gap: 5px;
          }

          p {
            flex-shrink: 0;
          }

          a {
            margin: 0;
          }
        }
    `}</style>
  </section>
);
