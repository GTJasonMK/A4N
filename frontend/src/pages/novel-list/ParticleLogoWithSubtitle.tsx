import React, { useEffect, useRef, useState } from "react";

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  targetX: number;
  targetY: number;
  size: number;
  opacity: number;
  color: string;
}

export const ParticleLogoWithSubtitle: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [isDark, setIsDark] = useState(false);

  // 检测暗色模式
  useEffect(() => {
    const checkDarkMode = () => {
      setIsDark(document.documentElement.classList.contains("dark"));
    };
    checkDarkMode();
    const observer = new MutationObserver(checkDarkMode);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);

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
    let mouse = { x: -1000, y: -1000, active: false };

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
      mouse.x = e.clientX - rect.left;
      mouse.y = e.clientY - rect.top;
      mouse.active = true;
    };

    const handleMouseLeave = () => {
      mouse.active = false;
    };

    canvas.addEventListener("mousemove", handleMouseMove);
    canvas.addEventListener("mouseleave", handleMouseLeave);

    // 提取 Logo 像素
    const extractLogoPixels = () => {
      const svgPath = "/resources/afn_svg_019c9984-b5e8-761f-9e57-62816676d110.svg";
      const img = new Image();

      img.onload = () => {
        // 创建临时 Canvas 渲染 SVG
        const tempCanvas = document.createElement("canvas");
        const tempCtx = tempCanvas.getContext("2d")!;
        const logoWidth = 300; // 再次增大到 200
        const logoHeight = 200;
        tempCanvas.width = logoWidth;
        tempCanvas.height = logoHeight;

        tempCtx.drawImage(img, 0, 0, logoWidth, logoHeight);
        const imageData = tempCtx.getImageData(0, 0, logoWidth, logoHeight);

        // Logo 位置：保留绘制缓冲，放在上半区偏中的位置
        const logoOffsetX = 32;
        const logoOffsetY = 20;

        // 采样 Logo 像素
        const sampling = 2;
        for (let y = 0; y < logoHeight; y += sampling) {
          for (let x = 0; x < logoWidth; x += sampling) {
            const i = (y * logoWidth + x) * 4;
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
                opacity: 0.8,
                color: `rgb(${r}, ${g}, ${b})`,
              });
            }
          }
        }

        animate();
      };

      img.src = svgPath;
    };

    let animationId: number;
    const animate = () => {
      ctx.clearRect(0, 0, width, height);

      particles.forEach((p) => {
        // 鼠标排斥力
        if (mouse.active) {
          const dx = p.x - mouse.x;
          const dy = p.y - mouse.y;
          const dist = Math.sqrt(dx * dx + dy * dy);

          if (dist < 80 && dist > 0) {
            const force = (80 - dist) / 80;
            p.vx += (dx / dist) * force * 0.5;
            p.vy += (dy / dist) * force * 0.5;
          }
        }

        // 回到目标位置
        const dx = p.targetX - p.x;
        const dy = p.targetY - p.y;
        p.vx += dx * 0.02;
        p.vy += dy * 0.02;

        // 阻尼
        applyEdgePressure(p);
        p.vx *= 0.9;
        p.vy *= 0.9;

        p.x += p.vx;
        p.y += p.vy;
        keepParticleInsideCanvas(p);

        // 绘制粒子
        ctx.fillStyle = p.color.replace("rgb", "rgba").replace(")", `, ${p.opacity})`);
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
        ctx.fill();
      });

      animationId = requestAnimationFrame(animate);
    };

    extractLogoPixels();

    return () => {
      cancelAnimationFrame(animationId);
      canvas.removeEventListener("mousemove", handleMouseMove);
      canvas.removeEventListener("mouseleave", handleMouseLeave);
    };
  }, [isDark]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: "100%", maxWidth: "360px", height: "auto", aspectRatio: "13 / 6" }}
      className="block cursor-pointer"
    />
  );
};
