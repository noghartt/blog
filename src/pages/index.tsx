import type { NextPage, GetStaticProps, GetStaticPaths } from 'next'

import { getAllPosts } from '../../lib/posts';
import type { Posts } from '../../lib/posts';

import { Container } from '../components/Container';
import { Header } from '../components/Header';
import { Article } from '../components/Article';

import { theme } from '../theme';


type Props = {
  posts: Posts;
}

const Home: NextPage<Props> = ({ posts }) => (
  <Container>
    <Header />
    <p>
      This place is to put some of my ideas and anotations about everything
      that I learn. A garden full of interesting subjects about computers, programming,
      math, economy, physics, or things like culinary, games, etc. Here is a place
      to put a lot of random notes about anything in the universe, a place to put
      these things to live forever.
    </p>
    <h2>Posts</h2>
    <section>
      {posts.map(({ metadata }) => (
        <Article
          key={metadata.id}
          title={metadata.title}
          date={metadata.date}
          href={`/posts/${metadata.slug}`}
          passHref
        />
      ))}
    </section>
    <style jsx>{`
      p {
        text-align: justify;
      }

      h2 {
        width: 100%;
        font-size: ${theme.fontSize.small};
      }

      section {
        display: flex;
        flex-direction: column;
        width: 100%;
        gap: 10px;
      }
    `}</style>
  </Container>
);

export const getStaticProps: GetStaticProps<Props> = async () => {
  const posts = await getAllPosts();

  return {
    props: {
      posts,
    },
  }
}

export default Home
