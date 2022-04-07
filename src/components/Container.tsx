export type ContainerProps = React.HTMLAttributes<HTMLDivElement>;

export const Container = ({ children, ...rest }: ContainerProps) => (
  <section {...rest}>
    {children}
    <style jsx>{`
      section {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-top: 4%;
        max-width: 40%;
        margin: 4% auto 0;
      }
    `}</style>
  </section>
)
