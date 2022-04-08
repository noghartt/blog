import 'normalize.css';
import type { AppProps } from 'next/app'

function MyApp({ Component, pageProps }: AppProps) {
  return (
    <>
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
