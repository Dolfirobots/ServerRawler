import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';

import Heading from '@theme/Heading';
import styles from './index.module.css';

const FeatureList = [
  {
    title: 'Performance',
    description: (
      <>
        Built with Rust, ServerRawler offers lightning-fast, efficient scanning
        of Minecraft servers, ensuring you gather data quickly and reliably.
      </>
    ),
  },
  {
    title: 'API Integration',
    description: (
      <>
        Easy-to-use API for seamless integration into your applications,
        allowing you to fetch and utilize Minecraft server data effortlessly.
      </>
    ),
  },
  {
    title: 'Customizability',
    description: (
      <>
        Highly configurable to suit your specific needs, for 
        targeted server crawling and data collection.
      </>
    ),
  },
];

function Feature({title, description}) {
  return (
    <div className={clsx('col col--4')}>
      <div className="feature-card">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">Discover the Minecraft Universe with ServerRawler</p>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg get-started-button"
            to="/docs/intro">
            Get Started
          </Link>
        </div>
      </div>
    </header>
  );
}

function HomepageFeatures() {
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

export default function Home() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title={`ServerRawler - ${siteConfig.tagline}`}
      description="Documentation for ServerRawler, a high-performance Minecraft server crawler written in Rust.">
      <HomepageHeader />
      <main>
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
