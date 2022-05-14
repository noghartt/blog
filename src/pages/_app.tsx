import 'normalize.css';
import Head from 'next/head';
import type { AppProps } from 'next/app'

function MyApp({ Component, pageProps }: AppProps) {
  return (
    <>
      <Head>
        <title>Noghartt&apos;s garden</title>
        <link rel="shortcut icon" href="/favicon.png" />
      </Head>
      <Component {...pageProps} />
      <style jsx global>{`
        html,
        body,
        #__next {
          display: flex;
          justify-content: center;
          height: 100%;
          width: 100%;
        }
      `}</style>
    </>
  );
}

export default MyApp
