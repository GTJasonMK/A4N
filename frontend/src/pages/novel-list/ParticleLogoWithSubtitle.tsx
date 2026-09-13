import React, { useEffect, useRef } from "react";
import afnLogoUrl from "../../assets/afn-logo.svg";
import { useI18n } from "../../i18n";

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  targetX: number;
  targetY: number;
  size: number;
  color: string;
}

export const ParticleLogoWithSubtitle: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const { t } = useI18n();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d", { alpha: true });
    if (!ctx) return;

    // 设置画布尺寸
    const width = 520;
    const height = 240;
    canvas.width = width;
    canvas.height = height;
    const edgePadding = 12;
    const edgeSoftZone = 28;

    const particles: Particle[] = [];
    const motionPreference = window.matchMedia("(prefers-reduced-motion: reduce)");
    const mouse = { x: -1000, y: -1000, active: false };
    let mouseIdleTimeout: number | null = null;
    const img = new Image();
    let animationId: number | null = null;
    let visible = true;

    const stopAnimation = () => {
      if (animationId != null) window.cancelAnimationFrame(animationId);
      animationId = null;
    };
    const startAnimation = () => {
      if (animationId != null || !particles.length || !visible || document.hidden || motionPreference.matches) return;
      animationId = window.requestAnimationFrame(animate);
    };
    const clearMouseIdleTimeout = () => {
      if (mouseIdleTimeout != null) window.clearTimeout(mouseIdleTimeout);
      mouseIdleTimeout = null;
    };
    const stopMouseInteraction = () => {
      mouseIdleTimeout = null;
      mouse.active = false;
      startAnimation();
    };

    const randomInsideCanvas = (span: number) => edgePadding + Math.random() * (span - edgePadding * 2);

    const keepParticleInsideCanvas = (p: Particle) => {
      const minX = edgePadding + p.size;
      const maxX = width - edgePadding - p.size;
      const minY = edgePadding + p.size;
      const maxY = height - edgePadding - p.size;

      if (p.x < minX) {
        p.x = minX;
        p.vx = Math.max(0, p.vx) * 0.35;
      } else if (p.x > maxX) {
        p.x = maxX;
        p.vx = Math.min(0, p.vx) * 0.35;
      }

      if (p.y < minY) {
        p.y = minY;
        p.vy = Math.max(0, p.vy) * 0.35;
      } else if (p.y > maxY) {
        p.y = maxY;
        p.vy = Math.min(0, p.vy) * 0.35;
      }
    };

    const applyEdgePressure = (p: Particle) => {
      const left = edgePadding + edgeSoftZone;
      const right = width - edgePadding - edgeSoftZone;
      const top = edgePadding + edgeSoftZone;
      const bottom = height - edgePadding - edgeSoftZone;

      if (p.x < left) p.vx += (left - p.x) * 0.01;
      else if (p.x > right) p.vx -= (p.x - right) * 0.01;

      if (p.y < top) p.vy += (top - p.y) * 0.01;
      else if (p.y > bottom) p.vy -= (p.y - bottom) * 0.01;
    };

    // 监听鼠标事件
    const handleMouseMove = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      // 计算鼠标在 canvas 坐标系中的位置（考虑 canvas 缩放）
      const scaleX = width / rect.width;
      const scaleY = height / rect.height;
      mouse.x = (e.clientX - rect.left) * scaleX;
      mouse.y = (e.clientY - rect.top) * scaleY;
      mouse.active = true;
      clearMouseIdleTimeout();
      mouseIdleTimeout = window.setTimeout(stopMouseInteraction, 180);
      startAnimation();
    };

    const handleMouseLeave = () => {
      clearMouseIdleTimeout();
      mouse.active = false;
      startAnimation();
    };

    canvas.addEventListener("mousemove", handleMouseMove);
    canvas.addEventListener("mouseleave", handleMouseLeave);

    // 提取 Logo 像素
    const extractLogoPixels = () => {
      img.onload = () => {
        // 创建临时 Canvas 渲染 SVG
        const tempCanvas = document.createElement("canvas");
        const tempCtx = tempCanvas.getContext("2d")!;
        const logoWidth = 300;
        const logoHeight = 200;
        tempCanvas.width = logoWidth;
        tempCanvas.height = logoHeight;

        tempCtx.drawImage(img, 0, 0, logoWidth, logoHeight);
        const imageData = tempCtx.getImageData(0, 0, logoWidth, logoHeight);

        // Logo 位置：水平居中，垂直偏上
        const logoOffsetX = (width - logoWidth) / 2;
        const logoOffsetY = 20;

        // 采样 Logo 像素
        const sampling = 2.5;
        for (let y = 0; y < logoHeight; y += sampling) {
          for (let x = 0; x < logoWidth; x += sampling) {
            const i = (Math.floor(y) * logoWidth + Math.floor(x)) * 4;
            const alpha = imageData.data[i + 3];

            if (alpha > 128) {
              const r = imageData.data[i];
              const g = imageData.data[i + 1];
              const b = imageData.data[i + 2];

              particles.push({
                x: randomInsideCanvas(width),
                y: randomInsideCanvas(height),
                vx: 0,
                vy: 0,
                targetX: logoOffsetX + x,
                targetY: logoOffsetY + y,
                size: Math.random() * 1.5 + 1,
                color: `rgba(${r}, ${g}, ${b}, 0.8)`,
              });
            }
          }
        }

        syncAnimation();
      };

      img.src = afnLogoUrl;
    };

    const drawParticles = () => {
      ctx.clearRect(0, 0, width, height);

      particles.forEach((p) => {
        ctx.fillStyle = p.color;
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
        ctx.fill();
      });
    };

    const animate = () => {
      animationId = null;
      let moving = false;

      particles.forEach((p) => {
        // 鼠标排斥力（增强效果）
        if (mouse.active) {
          const dx = p.x - mouse.x;
          const dy = p.y - mouse.y;
          const dist = Math.sqrt(dx * dx + dy * dy);
          const maxDist = 120; // 扩大交互范围

          if (dist < maxDist && dist > 0) {
            const force = (maxDist - dist) / maxDist;
            const strength = 1.2; // 增强排斥力度
            p.vx += (dx / dist) * force * strength;
            p.vy += (dy / dist) * force * strength;
          }
        }

        // 回到目标位置（降低吸引力，让鼠标交互更明显）
        const dx = p.targetX - p.x;
        const dy = p.targetY - p.y;
        p.vx += dx * 0.015;
        p.vy += dy * 0.015;

        // 阻尼（降低阻尼，让粒子运动更流畅）
        applyEdgePressure(p);
        p.vx *= 0.92;
        p.vy *= 0.92;

        p.x += p.vx;
        p.y += p.vy;
        keepParticleInsideCanvas(p);

        if (Math.abs(dx) + Math.abs(dy) + Math.abs(p.vx) + Math.abs(p.vy) > 0.1) {
          moving = true;
        } else {
          p.x = p.targetX;
          p.y = p.targetY;
          p.vx = 0;
          p.vy = 0;
        }
      });
      drawParticles();
      if (moving) startAnimation();
    };

    const syncAnimation = () => {
      stopAnimation();
      if (motionPreference.matches) {
        particles.forEach((particle) => {
          particle.x = particle.targetX;
          particle.y = particle.targetY;
          particle.vx = 0;
          particle.vy = 0;
        });
        drawParticles();
      } else {
        startAnimation();
      }
    };
    const visibilityObserver = new IntersectionObserver(([entry]) => {
      visible = entry.isIntersecting;
      syncAnimation();
    });
    visibilityObserver.observe(canvas);
    document.addEventListener("visibilitychange", syncAnimation);
    motionPreference.addEventListener("change", syncAnimation);
    extractLogoPixels();

    return () => {
      // StrictMode and route changes can dispose the canvas before the SVG loads.
      img.onload = null;
      clearMouseIdleTimeout();
      stopAnimation();
      visibilityObserver.disconnect();
      document.removeEventListener("visibilitychange", syncAnimation);
      motionPreference.removeEventListener("change", syncAnimation);
      canvas.removeEventListener("mousemove", handleMouseMove);
      canvas.removeEventListener("mouseleave", handleMouseLeave);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: "100%", maxWidth: "360px", height: "auto", aspectRatio: "13 / 6" }}
      className="block"
      role="img"
      aria-label={t("a11y.brandMark")}
    />
  );
};
