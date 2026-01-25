import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

const FeatureList = [
  {
    title: 'Easy to Use',
    image: require('@site/static/img/config_banner.jpg').default,
    description: (
     <>
        <small>
          Image:{' '}
          <a
            href="https://www.freepik.com"
            target="_blank"
            rel="noopener noreferrer"
          >
            Designed by vectorjuice / Freepik
          </a>
        </small>
        <br />
        ServerRawler is designed to be easy configure and installed.
      </>
    ),
  },
  {
    title: 'Discord integratation',
    Svg: require('@site/static/img/discord_logo.svg').default,
    description: (
      <>
        You can simple acces the ServerRawler with a Discord bot
      </>
    ),
  },
  {
    title: 'API',
    image: require('@site/static/img/api_banner.png').default,
    description: (
      <>
        If you want to control ServerRawler not with a Discord bot,
        you can use the Web API.
      </>
    ),
  },
];


function Feature({ Svg, image, title, description }) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center">
        <div className={styles.featureImage}>
          {Svg && <Svg role="img" />}
          {image && <img src={image} alt={title} />}
        </div>
      </div>

      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}


export default function HomepageFeatures() {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
