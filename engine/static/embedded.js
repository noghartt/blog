// A page shown inside a popup (a maximized note) hides the site's header and footer.
// Loaded in <head>, before anything is drawn, so they never flash.
if (window.self !== window.top) document.documentElement.classList.add('embedded');
