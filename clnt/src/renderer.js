// remove padding from sides of site, to make room for canvas
document.body.style.margin = "0px";
document.body.style.padding = "0px";
document.body.style.overflow = "hidden";
document.body.innerHTML += '<canvas id="canv"></canvas>'; 

var c = document.getElementById("canv");
c.width = window.innerWidth;
c.height = window.innerHeight;
c.style.width = window.innerWidth;//16;
c.style.height = window.innerHeight;//16;
c.style.imageRendering = "crisp-edges";

var ctx = c.getContext("2d");
ctx.imageSmoothingEnabled = false;
ctx.webkitImageSmoothingEnabled = false;
