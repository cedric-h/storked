// remove padding from sides of site, to make room for canvas
document.body.style.margin = "0px";
document.body.style.padding = "0px";
document.body.style.overflow = "hidden";
document.body.innerHTML += '<canvas id="canv"></canvas>'; 

var c = document.getElementById("canv");
c.width = window.innerWidth;
c.height = window.innerHeight;
c.style.width = window.innerWidth/2;
c.style.height = window.innerHeight/2;
c.style.imageRendering = "crisp-edges";

var ctx = c.getContext("2d");
ctx.imageSmoothingEnabled = false;
ctx.webkitImageSmoothingEnabled = false;

// this is used by the Rust part of things to read the cam movement.
// the movement is accumulated by JS event listeners, and when Rust reads it,
// it's cleared out. Rust calls the function that this closure returns.
let camMovement = (() => {
	let movementX = 0;
	let movementY = 0;

	// Hook pointer lock state change events for different browsers
	document.addEventListener('pointerlockchange', lockChangeAlert, false);
	document.addEventListener('mozpointerlockchange', lockChangeAlert, false);

	function lockChangeAlert() {
		if (document.pointerLockElement === c ||
			document.mozPointerLockElement === c) {
			console.log('The pointer lock status is now locked');
			document.addEventListener("mousemove", updatePosition, false);
		} else {
			console.log('The pointer lock status is now unlocked');  
			document.removeEventListener("mousemove", updatePosition, false);
		}
	}

	function updatePosition(e) {
		movementX += e.movementX;
		movementY += e.movementY;
	}

	// this is what's called when this var is called from Rust:
	return () => {
		let movement = [movementX, movementY];
		movementX = 0;
		movementY = 0;
		return movement;
	}
})();
// this grabs the mouse when the canvas is clicked.
c.onclick = function() {
  c.requestPointerLock();
};
