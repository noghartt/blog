import { Fragment } from "react";
import Link from "next/link";

import { theme } from '../theme';

const links = {
  home: '/',
  about: '/about',
  github:'https://github.com/noghartt/blog',
};

const Menu = () => (
  <nav>
    {Object.entries(links).map(([key, value], i, arr) => (
      <Fragment key={`item-${i}`}>
        {/* TODO: We could add it based on CSS' ::after pseudo-element, but
                  thinking in UX, this way can be better. I should study more
                  about it. */}
        {i > 0 && i < arr.length ? <span>/</span> : null}
        <Link href={value} replace>
          <a className="link">{key}</a>
        </Link>
      </Fragment>
    ))}
    <style jsx>{`
      nav {
        display: flex;
        align-items: center;
        gap: 10px;
        font-family: ${theme.fontFamily.monospace};
        font-size: ${theme.fontSize.small};
      }
    `}</style>
  </nav>
);

export const Header = () => (
  <header>
    <p>Noghartt</p>
    <Menu />

    <style jsx>{`
      header {
        display: flex;
        justify-content: space-between;
        width: 100%;
      }

      header > p {
        font-weight: 900;
        font-size: ${theme.fontSize.base};
      }

      @media (max-width: ${theme.breakpoints.small}) {
        header {
          flex-direction: column;
        }
      }
    `}</style>
  </header>
);
