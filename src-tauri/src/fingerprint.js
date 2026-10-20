(() => {
      if (window.__aeroPrivacyInstalled) return;
      Object.defineProperty(window, '__aeroPrivacyInstalled', { value: true });
      Object.defineProperty(Navigator.prototype, 'globalPrivacyControl', {
        configurable: false, get: () => true,
      });
      const originalCanvas = HTMLCanvasElement.prototype.toDataURL;
      HTMLCanvasElement.prototype.toDataURL = function(...args) {
        const context = this.getContext('2d');
        if (context && this.width && this.height) {
          try {
            const image = context.getImageData(0, 0, this.width, this.height);
            for (let index = 0; index < image.data.length; index += 64) image.data[index] ^= 1;
            context.putImageData(image, 0, 0);
          } catch {}
        }
        return originalCanvas.apply(this, args);
      };
      const originalGetParameter = WebGLRenderingContext.prototype.getParameter;
      WebGLRenderingContext.prototype.getParameter = function(parameter) {
        if (parameter === 37445) return 'Aero Graphics';
        if (parameter === 37446) return 'Generic Renderer';
        return originalGetParameter.call(this, parameter);
      };
      const originalFrequencyData = AnalyserNode.prototype.getFloatFrequencyData;
      AnalyserNode.prototype.getFloatFrequencyData = function(array) {
        originalFrequencyData.call(this, array);
        for (let index = 0; index < array.length; index += 32) array[index] += 0.00001;
      };
    })();