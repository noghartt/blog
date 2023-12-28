import { SITE } from '../config';
import type { CollectionEntry } from 'astro:content';
import dayjs from 'dayjs';

type Props = CollectionEntry<'blog'> & {
  tags?: boolean;
  date?: boolean;
}

export const og = ({ date = true, tags = true, ...props }: Props) => (
  <div
    style={{
      display: 'flex',
      backgroundColor: '#fff',
      height: '100%',
      width: '100%',
      padding: '6% 8%',
      position: 'relative',
    }}
  >
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        width: '100%',
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column' }}>
        <p style={{ fontSize: '12px', margin: 0, fontWeight: 300 }}>{SITE.website}</p>
        <p style={{ fontSize: '72px', fontWeight: 700, margin: 0 }}>{props.data.title}</p>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', width: '100%' }}>
        {tags ? (
          <p style={{ display: 'flex', gap: 8 }}>
            {props.data.tags.map((tag, i) => (
              <span key={i}>#{tag}</span>
            ))}
          </p>
        ) : null}
        {date ? <p>{dayjs(props.data.createdAt).format('YYYY-MM-DD')}</p> : null}
      </div>
    </div>
  </div>
);
