import { useDomElementState } from 'minimact-punch';

export default function Gallery() {
  const section = useDomElementState();

  return (
    <div id="gallery-root" ref={el => section.attachElement(el)}>
      <h2 id="gallery-title">Image Gallery</h2>

      {/* Lazy load when scrolled into view */}
      {section.isIntersecting && (
        <div id="gallery-images" className="images">
          <img src="photo1.jpg" alt="Photo 1" />
          <img src="photo2.jpg" alt="Photo 2" />
        </div>
      )}

      {/* Show collapse button when too many children */}
      {section.childrenCount > 5 && (
        <button id="collapse-btn" type="button">
          Collapse
        </button>
      )}

      {/* Show badge if intersecting */}
      {section.isIntersecting && (
        <span id="visible-badge" className="badge">
          Visible
        </span>
      )}
    </div>
  );
}
