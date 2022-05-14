import { Container } from '../components/Container';
import { Header } from '../components/Header';

const myContacts = {
  github: 'https://github.com/noghartt',
  twitter: 'https://twitter.com/noghartt',
  devto: 'https://dev.to/noghartt',
  email: 'mailto:me@noghartt.dev',
}

const About = () => (
  <Container>
    <Header />
    <article>
      <p>
        I&apos;m a brazilian software engineer from Brazil. Programming for about seven years
        of my life. Professionaly working with JavaScript (React and other drugs) but with
        a lot of interest in a more formal computer subjects: type theory, PL theory, etc.
      </p>
      <p>
        If I remember other things to put here, I&apos;ll update.
      </p>
      <p>
        <strong>My contacts</strong>
      </p>
      <section>
        {Object.entries(myContacts).map(([key, value]) => (
          <a key={key} href={value} target="_blank" rel="noreferrer">{key}</a>
        ))}
      </section>
      <style jsx>{`
        section {
          display: flex;
          justify-content: space-between;
        }
      `}</style>
    </article>
  </Container>
);

export default About;
