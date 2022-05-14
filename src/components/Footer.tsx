export const Footer = () => (
  <footer>
    <p>
      <em>Do you have any feedback or suggestion to improve this post?</em>
      <br />
      <em>
        Please, send me an
        {' '}
        <a href="mailto:me@noghartt.dev">email</a>{' '}
        or a{' '}
        <a href="https://github.com/noghartt/blog">PR in GitHub</a>.
      </em>
    </p>
    <style jsx>{`
      p {
        text-align: center;
      }
    `}</style>
  </footer>
);
