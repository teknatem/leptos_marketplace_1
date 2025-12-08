// Particle animation for login page background
// Adapted from erp-flow-hub

(function() {
    'use strict';
    
    function initParticleAnimation() {
        const canvas = document.getElementById('particle-canvas');
        if (!canvas) return;
        
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        
        // Resize canvas to fill window
        const resizeCanvas = () => {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
        };
        
        resizeCanvas();
        window.addEventListener('resize', resizeCanvas);
        
        // Particle configuration - adapting indigo/violet colors
        const particles = [];
        const particleCount = 50;
        const connectionDistance = 150;
        
        // Create particles with indigo/violet colors
        for (let i = 0; i < particleCount; i++) {
            particles.push({
                x: Math.random() * canvas.width,
                y: Math.random() * canvas.height,
                vx: (Math.random() - 0.5) * 0.5,
                vy: (Math.random() - 0.5) * 0.5,
                size: Math.random() * 2 + 1,
                opacity: Math.random() * 0.5 + 0.2
            });
        }
        
        // Animation loop
        const animate = () => {
            // Fade effect instead of clear (creates trailing effect)
            // Higher opacity = less trailing (0.3 is good balance)
            // Using blue gradient background colors
            ctx.fillStyle = 'rgba(44, 82, 130, 0.3)'; // Darker blue with higher opacity
            ctx.fillRect(0, 0, canvas.width, canvas.height);
            
            particles.forEach((particle, i) => {
                // Update position
                particle.x += particle.vx;
                particle.y += particle.vy;
                
                // Bounce off edges
                if (particle.x < 0 || particle.x > canvas.width) particle.vx *= -1;
                if (particle.y < 0 || particle.y > canvas.height) particle.vy *= -1;
                
                // Draw particle (blue color to match app: rgb(0, 123, 255))
                ctx.beginPath();
                ctx.arc(particle.x, particle.y, particle.size, 0, Math.PI * 2);
                ctx.fillStyle = `rgba(0, 123, 255, ${particle.opacity})`;
                ctx.fill();
                
                // Connect nearby particles with lines
                particles.slice(i + 1).forEach((p2) => {
                    const dx = particle.x - p2.x;
                    const dy = particle.y - p2.y;
                    const distance = Math.sqrt(dx * dx + dy * dy);
                    
                    if (distance < connectionDistance) {
                        ctx.beginPath();
                        ctx.moveTo(particle.x, particle.y);
                        ctx.lineTo(p2.x, p2.y);
                        const lineOpacity = 0.15 * (1 - distance / connectionDistance);
                        ctx.strokeStyle = `rgba(0, 123, 255, ${lineOpacity})`;
                        ctx.lineWidth = 0.5;
                        ctx.stroke();
                    }
                });
            });
            
            requestAnimationFrame(animate);
        };
        
        animate();
        
        // Cleanup
        return () => {
            window.removeEventListener('resize', resizeCanvas);
        };
    }
    
    // Initialize when DOM is ready
    // Use setTimeout to ensure Leptos has rendered the canvas element
    const tryInit = () => {
        const canvas = document.getElementById('particle-canvas');
        if (canvas) {
            initParticleAnimation();
        } else {
            // Canvas not ready yet, try again in 100ms
            setTimeout(tryInit, 100);
        }
    };
    
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', tryInit);
    } else {
        tryInit();
    }
})();

