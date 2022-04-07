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

        section > p {
          margin: 0;
        }

        .article-date {
          font-style: italic;
          font-size: ${theme.fontSize.base}
        }

        .article-title {
          font-size: ${theme.fontSize.base};
          font-family: ${theme.fontFamily.monospace};
          text-decoration: underline;
        }
      `}</style>
  </section>
);
