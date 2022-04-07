import type { NextPage, GetStaticProps, GetStaticPaths } from 'next';
import { useRouter } from 'next/router';

import { getAllPosts, getPostBySlug } from '../../../lib/posts';
import type { Post } from '../../../lib/posts';

import { Container } from '../../components/Container';
import { Header } from '../../components/Header';

import { theme } from '../../theme';
import { Markdown } from '../../components/Markdown';

type Props = {
  post: Post;
}

type QueryParams = {
  slug?: string;
};

const Post: NextPage<Props> = ({ post }) => (
  <Container>
    <Header />
    <Markdown post={post} />
  </Container>
);

export default Post;

export const getStaticPaths: GetStaticPaths = async () => {
  const posts = await getAllPosts();

  const paths = posts.map(post => ({ params: { slug: post.metadata.slug } }));

  return { paths, fallback: false };
}

export const getStaticProps: GetStaticProps<Props, QueryParams> = async ({ params }) => {
  const post = await getPostBySlug(params?.slug);

  // TODO: Is this conditional really necessary?
  if (!post) {
    throw new Error("This post doesn't exist! Please, try again with some correct slug.!");
  }

  return {
    props: {
      post,
    }
  }
}
