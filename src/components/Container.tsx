import { theme } from "../theme";

export type ContainerProps = React.HTMLAttributes<HTMLDivElement>;

export const Container = ({ children, ...rest }: ContainerProps) => (
  <section {...rest}>
    {children}
    <style jsx>{`
      section {
        display: flex;
        flex-direction: column;
        align-items: center;
        max-width: 80%;
        margin-top: 4%;
      }

      @media (min-width: ${theme.breakpoints.large}) {
        section {
          max-width: 40%;
        }
      }
    `}</style>
  </section>
);
