export default function () {
  if (typeof document === 'undefined') return;

  const baseHref = (document.querySelector('base') && document.querySelector('base').getAttribute('href')) || '/';
  const href = new URL('img/favicon.png', baseHref).toString();

  function setIcon(h) {
    const rels = ['icon', 'shortcut icon'];
    rels.forEach((rel) => {
      let link = document.querySelector(`link[rel="${rel}"]`);
      if (!link) {
        link = document.createElement('link');
        link.setAttribute('rel', rel);
        document.head.appendChild(link);
      }
      link.setAttribute('href', h);
      link.setAttribute('type', 'image/png');
      link.setAttribute('sizes', '32x32');
    });
  }

  setIcon(href);

  window.setFavicon = setIcon;
}
