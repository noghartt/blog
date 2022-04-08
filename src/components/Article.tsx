import Link from "next/link";
import { LinkProps } from "next/link";

import { theme } from "../theme";

export type ArticleProps = LinkProps & {
  title: string;
  date: string;
};

export const Article = ({ title, date, ...linkProps }: ArticleProps) => (
  <section>
    <p>{date}</p>
    <Link {...linkProps}>
      <a>{title}</a>
    </Link>
    <style jsx>{`
        section {
          display: flex;
          gap: 20px;
          width: 100%;
        }

        p {
          margin: 0;
          font-style: italic;
          font-size: ${theme.fontSize.base};
          white-space: nowrap;
        }

        a {
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
