// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
// MIT License
//
// One ROM Logic Analyzer - Browser-based PIO emulator visualization

'use strict';

// =============================================================================
// CONFIGURATION AND CONSTANTS
// =============================================================================

const CONFIG = {
    // Timebase configuration
    CYCLES_PER_PIXEL_MIN: 0.01,      // Minimum cycles per pixel (zoomed in)
    CYCLES_PER_PIXEL_MAX: 1000,      // Maximum cycles per pixel (zoomed out)
    CYCLES_PER_PIXEL_DEFAULT: 1.0,   // Default zoom level
    
    // Slider uses log scale mapping: slider value maps to actual cycles/pixel
    // slider = -200 to +300 maps to 0.01 to 1000 cycles/pixel logarithmically
    SLIDER_MIN: -200,
    SLIDER_MAX: 300,
    
    // Pin drive values
    PIN_DRIVEN_LOW: 0,
    PIN_DRIVEN_HIGH: 1,
    PIN_NOT_DRIVEN: 255,
    
    // Waveform rendering
    TRACE_HEIGHT: 20,                // Height of each signal trace in pixels
    TRACE_SPACING: 4,                // Vertical spacing between traces
    HEX_TRACE_HEIGHT: 24,            // Height of hex summary traces
    FONT_SIZE: 12,                   // Font size for labels and hex values
    LABEL_WIDTH: 60,                 // Width reserved for signal labels
    
    // Execution
    DEFAULT_SPEED: 100,              // Default cycles per frame
    
    // Auto-scroll margin (pixels from right edge where we start scrolling)
    AUTOSCROLL_MARGIN: 100,

    // Cursor
    CURSOR_VALUE_FONT_SIZE: 20,     // Font size for cursor value display
    CURSOR_VALUE_OFFSET_X: 20,      // Horizontal offset from cursor line
    CURSOR_VALUE_OFFSET_Y: -0.5,       // Vertical offset above trace (as fraction of trace height)
    CURSOR_VALUE_COLOR: '#000000',  // Color for cursor values};
    CURSOR_VALUE_TEXT_COLOR: '#ffffff',  // Color for cursor values
    CURSOR_VALUE_PADDING_X: 2,      // Horizontal padding around cursor value text
    CURSOR_VALUE_PADDING_Y: 2,      // Vertical padding around cursor value text
    CURSOR_VALUE_HEIGHT_MULTIPLIER: 1.3,  // Account for font ascenders
}

// Get CSS color variables
function getCSSColor(varName) {
    return getComputedStyle(document.documentElement).getPropertyValue(varName).trim();
}

// Color cache - populated on first render
const COLORS = {};

// =============================================================================
// WASM MODULE WRAPPER
// =============================================================================

class WASMModule {
    constructor() {
        this.module = null;
        this.ready = false;
        this.epioHandle = null;  // Store the epio_t* pointer
    }
    
    async init() {
        return new Promise((resolve, reject) => {
            // Check if module is already initialized
            if (Module.calledRun) {
                this.module = Module;
                this.ready = true;
                resolve();
                return;
            }
            
            // Set up callback for when it initializes
            Module.onRuntimeInitialized = () => {
                this.module = Module;
                this.ready = true;
                resolve();
            };
        });
    }
    
    // High-level onerom API
    oneromInit() {
        // Returns epio_t* handle (as a number)
        this.epioHandle = this.module.ccall('onerom_init', 'number', [], []);
        return this.epioHandle !== 0;  // Return success/fail as boolean
    }
    
    oneromDrivePins(addr, numAddrBits, cs1, cs2, cs3, x1, x2) {
        this.module.ccall('onerom_drive_pins', null,
            ['number', 'number', 'number', 'number', 'number', 'number', 'number'],
            [addr, numAddrBits, cs1, cs2, cs3, x1, x2]);
    }
    
    oneromReleasePins() {
        this.module.ccall('onerom_release_pins', null, [], []);
    }
    
    oneromReadData(dataBits) {
        return this.module.ccall('onerom_read_data', 'number', ['number'], [dataBits]);
    }
    
    // Pin mapping API
    oneromGetAddrPin(bit) {
        return this.module.ccall('onerom_get_addr_pin', 'number', ['number'], [bit]);
    }
    
    oneromGetDataPin(bit) {
        return this.module.ccall('onerom_get_data_pin', 'number', ['number'], [bit]);
    }
    
    oneromGetCS1Pin() {
        return this.module.ccall('onerom_get_cs1_pin', 'number', [], []);
    }
    
    oneromGetCS2Pin() {
        return this.module.ccall('onerom_get_cs2_pin', 'number', [], []);
    }
    
    oneromGetCS3Pin() {
        return this.module.ccall('onerom_get_cs3_pin', 'number', [], []);
    }
    
    oneromGetX1Pin() {
        return this.module.ccall('onerom_get_x1_pin', 'number', [], []);
    }
    
    oneromGetX2Pin() {
        return this.module.ccall('onerom_get_x2_pin', 'number', [], []);
    }
    
    // Direct epio API calls using the handle
    epioStepCycles(cycles) {
        this.module.ccall('epio_step_cycles', null, ['number', 'number'], 
            [this.epioHandle, cycles]);
    }
    
    epioGetCycleCount() {
        return BigInt(this.module.ccall('epio_get_cycle_count', 'number', ['number'], 
            [this.epioHandle]));
    }
    
    epioResetCycleCount() {
        this.module.ccall('epio_reset_cycle_count', null, ['number'], 
            [this.epioHandle]);
    }
    
    epioReadGPIOsExt() {
        return BigInt(this.module.ccall('epio_read_gpios_ext', 'number', ['number'], 
            [this.epioHandle]));
    }

    epioReadPinStates() {
        return BigInt(this.module.ccall('epio_read_pin_states', 'number', ['number'], 
            [this.epioHandle]));
    }

    epioReadDrivenPins() {
        return BigInt(this.module.ccall('epio_read_driven_pins', 'number', ['number'], 
            [this.epioHandle]));
    }
}

// =============================================================================
// SAMPLE BUFFER
// =============================================================================

class SampleBuffer {
    constructor() {
        this.samples = [];  // Array of {cycle: BigInt, gpios: BigInt, driven: BigInt}
    }
    
    addSample(cycle, gpios, driven) {
        this.samples.push({cycle, gpios, driven});
    }
    
    clear() {
        this.samples = [];
    }
    
    getSamplesInRange(startCycle, endCycle) {
        // Binary search to find start index
        let start = 0;
        let end = this.samples.length;
        
        while (start < end) {
            const mid = Math.floor((start + end) / 2);
            if (this.samples[mid].cycle < startCycle) {
                start = mid + 1;
            } else {
                end = mid;
            }
        }
        
        // Collect samples in range
        const result = [];
        for (let i = start; i < this.samples.length; i++) {
            if (this.samples[i].cycle >= endCycle) break;
            result.push(this.samples[i]);
        }
        
        return result;
    }
    
    get length() {
        return this.samples.length;
    }
    
    get lastCycle() {
        return this.samples.length > 0 ? 
            this.samples[this.samples.length - 1].cycle : 0n;
    }
}

// =============================================================================
// SIGNAL DECODER
// =============================================================================

class SignalDecoder {
    constructor(wasmModule) {
        this.wasm = wasmModule;
        this.pinMap = null;
    }
    
    // Build pin mapping from WASM
    buildPinMap(numAddrBits) {
        this.pinMap = {
            addr: [],
            data: [],
            control: {
                cs1: this.wasm.oneromGetCS1Pin(),
                cs2: this.wasm.oneromGetCS2Pin(),
                cs3: this.wasm.oneromGetCS3Pin(),
                x1: this.wasm.oneromGetX1Pin(),
                x2: this.wasm.oneromGetX2Pin(),
            }
        };
        
        // Address pins
        for (let i = 0; i < numAddrBits; i++) {
            this.pinMap.addr[i] = this.wasm.oneromGetAddrPin(i);
        }
        
        // Data pins (always 8 bits for now)
        for (let i = 0; i < 8; i++) {
            this.pinMap.data[i] = this.wasm.oneromGetDataPin(i);
        }
    }
    
    // Extract bit state from GPIO value
    extractBit(gpios, pinNum) {
        return (gpios & (1n << BigInt(pinNum))) !== 0n ? 1 : 0;
    }
    
    // Decode address value from GPIO state
    decodeAddress(gpios) {
        let addr = 0;
        for (let i = 0; i < this.pinMap.addr.length; i++) {
            if (this.extractBit(gpios, this.pinMap.addr[i])) {
                addr |= (1 << i);
            }
        }
        return addr;
    }
    
    // Decode data value from GPIO state
    decodeData(gpios) {
        let data = 0;
        for (let i = 0; i < this.pinMap.data.length; i++) {
            if (this.extractBit(gpios, this.pinMap.data[i])) {
                data |= (1 << i);
            }
        }
        return data;
    }
    
    // Decode control signal
    decodeControl(gpios, controlName) {
        const pinNum = this.pinMap.control[controlName];
        return this.extractBit(gpios, pinNum);
    }

    // Check if pin is actively driven
    isPinDriven(driven, pinNum) {
        return (driven & (1n << BigInt(pinNum))) !== 0n;
    }
}

// =============================================================================
// EXECUTION ENGINE
// =============================================================================

class ExecutionEngine {
    constructor(wasmModule, sampleBuffer) {
        this.wasm = wasmModule;
        this.samples = sampleBuffer;
        
        // Execution state
        this.running = false;
        this.paused = false;
        this.animationId = null;
        
        // Read sequence state machine
        this.currentAddr = 0;
        this.maxAddr = 0;
        this.readState = 'idle';
        this.cyclesRemaining = 0;
        
        // Store only what doesn't change during execution
        this.numAddrBits = 13;
        this.cyclesPerFrame = CONFIG.DEFAULT_SPEED;
    }

    // Start a complete read sequence
    startCompleteRead(numAddrBits) {
        if (this.running) return;
        
        this.numAddrBits = numAddrBits;
        this.currentAddr = 0;
        this.maxAddr = (1 << numAddrBits) - 1;
        this.readState = 'drive';
        this.cyclesRemaining = 0;
        
        this.running = true;
        this.paused = false;
        this.animate();
    }
    
    pause() {
        this.paused = true;
    }
    
    resume() {
        if (!this.running) return;
        this.paused = false;
        this.animate();
    }
    
    stop() {
        this.running = false;
        this.paused = false;
        this.readState = 'idle';
        if (this.animationId) {
            cancelAnimationFrame(this.animationId);
            this.animationId = null;
        }
    }
    
    // Animation loop
    animate() {
        if (!this.running || this.paused) return;
        
        // Execute cycles for this frame
        for (let i = 0; i < this.cyclesPerFrame; i++) {
            if (!this.stepOneCycle()) {
                // Sequence complete
                this.stop();
                return;
            }
        }
        
        // Continue animation
        this.animationId = requestAnimationFrame(() => this.animate());
    }
    
    // Step one PIO cycle and update state machine
    stepOneCycle() {
        if (this.currentAddr > this.maxAddr) {
            return false;  // Complete
        }
        
        // Read current values from UI
        const setupCycles = parseInt(document.getElementById('setupCycles').value);
        const recoveryCycles = parseInt(document.getElementById('recoveryCycles').value);
        
        switch (this.readState) {
            case 'drive':
                this.wasm.oneromDrivePins(
                    this.currentAddr,
                    this.numAddrBits,
                    CONFIG.PIN_DRIVEN_LOW,
                    CONFIG.PIN_NOT_DRIVEN,
                    CONFIG.PIN_NOT_DRIVEN,
                    CONFIG.PIN_NOT_DRIVEN,
                    CONFIG.PIN_NOT_DRIVEN
                );
                this.readState = 'setup';
                this.cyclesRemaining = setupCycles;
                break;
                
            case 'setup':
                if (--this.cyclesRemaining === 0) {
                    this.wasm.oneromReadData(8);
                    this.readState = 'release';
                }
                break;
                
            case 'release':
                this.wasm.oneromReleasePins();
                this.readState = 'recovery';
                this.cyclesRemaining = recoveryCycles;
                break;
                
            case 'recovery':
                if (--this.cyclesRemaining === 0) {
                    this.currentAddr++;
                    if (this.currentAddr <= this.maxAddr) {
                        this.readState = 'drive';
                    }
                }
                break;
        }
        
        // Step PIO and capture sample
        this.wasm.epioStepCycles(1);
        const cycle = this.wasm.epioGetCycleCount();
        const gpios = this.wasm.epioReadPinStates();
        const driven = this.wasm.epioReadDrivenPins();
        this.samples.addSample(cycle, gpios, driven);
        
        return true;
    }
    
    // Get progress percentage
    getProgress() {
        if (this.maxAddr === 0) return 0;
        return Math.floor(100 * this.currentAddr / (this.maxAddr + 1));
    }
    
    isRunning() {
        return this.running;
    }
    
    isPaused() {
        return this.paused;
    }
}

// =============================================================================
// WAVEFORM RENDERER
// =============================================================================

class WaveformRenderer {
    constructor(canvas) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.cyclesPerPixel = CONFIG.CYCLES_PER_PIXEL_DEFAULT;
        this.scrollPos = 0;  // In cycles
        this.autoScroll = true;
        
        // Signal group visibility
        this.showAddr = true;
        this.showData = true;
        this.showControl = true;
        
        // Signal group expansion state
        this.addrExpanded = true;
        this.dataExpanded = true;
        
        this.cursorX = null;  // Mouse X position for cursor
        this.cursorCycle = null;  // Cycle at cursor position

        // Load colors from CSS
        this.loadColors();
    }
    
    loadColors() {
        COLORS.background = getCSSColor('--color-background');
        COLORS.grid = getCSSColor('--color-grid');
        COLORS.high = getCSSColor('--color-high');
        COLORS.low = getCSSColor('--color-low');
        COLORS.smearedHigh = getCSSColor('--color-smeared-high');
        COLORS.smearedLow = getCSSColor('--color-smeared-low');
        COLORS.smearedHatch = getCSSColor('--color-smeared-hatch');
        COLORS.hex = getCSSColor('--color-hex');
        COLORS.label = getCSSColor('--color-label');
        COLORS.groupBg = getCSSColor('--color-group-bg');
    }
    
    resize() {
        const container = this.canvas.parentElement;
        this.canvas.width = container.clientWidth;
        this.canvas.height = container.clientHeight;
    }
    
    clear() {
        this.ctx.fillStyle = COLORS.background;
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    }
    
    // Convert cycle number to pixel X coordinate
    cycleToPixelX(cycle) {
        return CONFIG.LABEL_WIDTH + (Number(cycle) / this.cyclesPerPixel) - this.scrollPos;
    }
    
    // Update scroll position for auto-scroll
    updateAutoScroll(lastCycle) {
        if (!this.autoScroll) return;
        
        const waveformWidth = this.canvas.width - CONFIG.LABEL_WIDTH;
        const lastCyclePixel = Number(lastCycle) / this.cyclesPerPixel;
        const currentViewEnd = this.scrollPos + waveformWidth;
        
        // If we're within margin of right edge, scroll to keep following
        if (lastCyclePixel > currentViewEnd - CONFIG.AUTOSCROLL_MARGIN) {
            this.scrollPos = Math.max(0, lastCyclePixel - waveformWidth + CONFIG.AUTOSCROLL_MARGIN);
        }
    }
    
    // Calculate Y position for each trace
    calculateTraceLayout(decoder) {
        const MIN_TRACE_HEIGHT = 20;
        const MIN_HEX_TRACE_HEIGHT = 24;
        const MIN_TRACE_SPACING = 4;
        
        // First pass - calculate minimum required height
        let minTotalHeight = MIN_TRACE_SPACING;
        const traceList = [];
        
        // Address group
        if (this.showAddr) {
            traceList.push({
                type: 'addr_hex',
                minHeight: MIN_HEX_TRACE_HEIGHT,
                label: `A[${decoder.pinMap.addr.length-1}:0]`
            });
            minTotalHeight += MIN_HEX_TRACE_HEIGHT + MIN_TRACE_SPACING;
            
            if (this.addrExpanded) {
                for (let i = decoder.pinMap.addr.length - 1; i >= 0; i--) {
                    traceList.push({
                        type: 'addr_bit',
                        bit: i,
                        minHeight: MIN_TRACE_HEIGHT,
                        label: `A${i}`
                    });
                    minTotalHeight += MIN_TRACE_HEIGHT + MIN_TRACE_SPACING;
                }
            }
        }
        
        // Data group
        if (this.showData) {
            traceList.push({
                type: 'data_hex',
                minHeight: MIN_HEX_TRACE_HEIGHT,
                label: `D[${decoder.pinMap.data.length-1}:0]`
            });
            minTotalHeight += MIN_HEX_TRACE_HEIGHT + MIN_TRACE_SPACING;
            
            if (this.dataExpanded) {
                for (let i = decoder.pinMap.data.length - 1; i >= 0; i--) {
                    traceList.push({
                        type: 'data_bit',
                        bit: i,
                        minHeight: MIN_TRACE_HEIGHT,
                        label: `D${i}`
                    });
                    minTotalHeight += MIN_TRACE_HEIGHT + MIN_TRACE_SPACING;
                }
            }
        }
        
        // Control signals
        if (this.showControl) {
            const controls = ['cs1', 'cs2', 'cs3', 'x1', 'x2'];
            for (const ctrl of controls) {
                traceList.push({
                    type: 'control',
                    signal: ctrl,
                    minHeight: MIN_TRACE_HEIGHT,
                    label: ctrl.toUpperCase()
                });
                minTotalHeight += MIN_TRACE_HEIGHT + MIN_TRACE_SPACING;
            }
        }
        
        // Calculate scale factor (at least 1.0, never shrink below minimum)
        const availableHeight = this.canvas.height;
        const scaleFactor = Math.max(1.0, availableHeight / minTotalHeight);
        
        // Second pass - apply scaling and calculate positions
        const layout = [];
        let y = MIN_TRACE_SPACING * scaleFactor;
        
        for (const trace of traceList) {
            layout.push({
                ...trace,
                y: y,
                height: trace.minHeight * scaleFactor
            });
            y += (trace.minHeight + MIN_TRACE_SPACING) * scaleFactor;
        }
        
        return layout;
    }

    // Render all waveforms
    render(sampleBuffer, decoder) {
        this.clear();
        
        const layout = this.calculateTraceLayout(decoder);
        
        // Draw labels
        this.ctx.fillStyle = COLORS.label;
        this.ctx.font = `${CONFIG.FONT_SIZE}px monospace`;
        this.ctx.textAlign = 'right';
        this.ctx.textBaseline = 'middle';
        
        for (const trace of layout) {
            this.ctx.fillText(trace.label, CONFIG.LABEL_WIDTH - 5, trace.y + trace.height / 2);
        }
        
        // Calculate visible cycle range
        const waveformWidth = this.canvas.width - CONFIG.LABEL_WIDTH;
        const startCycle = BigInt(Math.floor(this.scrollPos * this.cyclesPerPixel));
        const endCycle = BigInt(Math.floor((this.scrollPos + waveformWidth) * this.cyclesPerPixel));
        
        // Get all samples in visible range
        const visibleSamples = sampleBuffer.getSamplesInRange(startCycle, endCycle);
        if (visibleSamples.length === 0) return;
        
        // Render each trace
        for (const trace of layout) {
            if (trace.type === 'addr_hex') {
                this.renderHexTrace(trace, visibleSamples, decoder, 'addr', waveformWidth);
            } else if (trace.type === 'data_hex') {
                this.renderHexTrace(trace, visibleSamples, decoder, 'data', waveformWidth);
            } else if (trace.type === 'addr_bit') {
                const pinNum = decoder.pinMap.addr[trace.bit];
                this.renderBitTrace(trace, visibleSamples, decoder, pinNum, waveformWidth);
            } else if (trace.type === 'data_bit') {
                const pinNum = decoder.pinMap.data[trace.bit];
                this.renderBitTrace(trace, visibleSamples, decoder, pinNum, waveformWidth);
            } else if (trace.type === 'control') {
                const pinNum = decoder.pinMap.control[trace.signal];
                this.renderBitTrace(trace, visibleSamples, decoder, pinNum, waveformWidth);
            }
        }

        // Draw cursor if active
        if (this.cursorX !== null && this.cursorCycle !== null) {
            this.renderCursor(sampleBuffer, decoder, layout);
        }
    }
    
    // Render a single bit trace - draws horizontal lines and vertical edges
    renderBitTrace(trace, samples, decoder, pinNum, waveformWidth) {
        if (samples.length === 0) return;
        
        const yHigh = trace.y + 3;
        const yLow = trace.y + trace.height - 3;
        const canvasEndX = CONFIG.LABEL_WIDTH + waveformWidth;
        
        let previousState = null;
        let previousX = null;
        let previousDriven = null;
        
        this.ctx.lineWidth = 1;
        
        for (const sample of samples) {
            const currentState = decoder.extractBit(sample.gpios, pinNum);
            const isDriven = decoder.isPinDriven(sample.driven, pinNum);
            const currentX = this.cycleToPixelX(sample.cycle);
            
            // Skip if off-screen to the left
            if (currentX < CONFIG.LABEL_WIDTH) {
                previousState = currentState;
                previousDriven = isDriven;
                previousX = currentX;
                continue;
            }
            
            // Stop if off-screen to the right
            if (currentX > canvasEndX) break;
            
            if (previousState !== null && previousX !== null) {
                // Draw horizontal line from previous to current
                this.ctx.beginPath();
                
                if (previousDriven) {
                    // Driven - draw at HIGH or LOW
                    this.ctx.strokeStyle = COLORS.high;
                    const y = previousState ? yHigh : yLow;
                    this.ctx.moveTo(Math.max(CONFIG.LABEL_WIDTH, previousX), y);
                    this.ctx.lineTo(currentX, y);
                } else {
                    // High-Z - draw at middle level
                    this.ctx.strokeStyle = COLORS.smearedHatch;
                    const yMid = trace.y + trace.height / 2;
                    this.ctx.moveTo(Math.max(CONFIG.LABEL_WIDTH, previousX), yMid);
                    this.ctx.lineTo(currentX, yMid);
                }
                
                this.ctx.stroke();
                
                // If drive state or level changed, draw transition
                if (isDriven !== previousDriven || (isDriven && currentState !== previousState)) {
                    this.ctx.beginPath();
                    this.ctx.strokeStyle = COLORS.high;
                    
                    // Calculate previous and current Y positions
                    let prevY, currY;
                    
                    if (previousDriven) {
                        prevY = previousState ? yHigh : yLow;
                    } else {
                        prevY = trace.y + trace.height / 2; // yMid
                    }
                    
                    if (isDriven) {
                        currY = currentState ? yHigh : yLow;
                    } else {
                        currY = trace.y + trace.height / 2; // yMid
                    }
                    
                    this.ctx.moveTo(currentX, prevY);
                    this.ctx.lineTo(currentX, currY);
                    this.ctx.stroke();
                }
            }
            
            previousState = currentState;
            previousDriven = isDriven;
            previousX = currentX;
        }
        
        // Draw final horizontal line, but only if last sample is visible
        if (previousState !== null && previousX !== null && previousX < canvasEndX) {
            const lastSampleX = this.cycleToPixelX(samples[samples.length - 1].cycle);
            if (lastSampleX > previousX) {
                this.ctx.beginPath();
                if (previousDriven) {
                    this.ctx.strokeStyle = COLORS.high;
                    const y = previousState ? yHigh : yLow;
                    this.ctx.moveTo(previousX, y);
                    this.ctx.lineTo(Math.min(lastSampleX, canvasEndX), y);
                } else {
                    this.ctx.strokeStyle = COLORS.smearedHatch;
                    const yMid = trace.y + trace.height / 2;
                    this.ctx.moveTo(previousX, yMid);
                    this.ctx.lineTo(Math.min(lastSampleX, canvasEndX), yMid);
                }
                this.ctx.stroke();
            }
        }
    }
    
    // Render hex summary trace
    renderHexTrace(trace, samples, decoder, type, waveformWidth) {
        if (samples.length === 0) return;
        
        const bits = type === 'addr' ? decoder.pinMap.addr : decoder.pinMap.data;
        const canvasEndX = CONFIG.LABEL_WIDTH + waveformWidth;
        
        // Track value changes
        let previousValue = null;
        let previousX = null;
        let stableStartX = null;
        let stableValue = null;
        
        for (let i = 0; i < samples.length; i++) {
            const sample = samples[i];
            const currentValue = type === 'addr' ? 
                decoder.decodeAddress(sample.gpios) : 
                decoder.decodeData(sample.gpios);
            const currentX = this.cycleToPixelX(sample.cycle);
            
            // Skip if off-screen to the left
            if (currentX < CONFIG.LABEL_WIDTH) {
                previousValue = currentValue;
                previousX = currentX;
                stableValue = currentValue;
                stableStartX = currentX;
                continue;
            }
            
            // Stop if off-screen to the right
            if (currentX > canvasEndX) break;
            
            if (previousValue !== null) {
                if (currentValue === previousValue) {
                    // Value still stable, continue
                } else {
                    // Value changed - draw the stable region if it was stable
                    if (stableValue !== null && stableStartX !== null) {
                        const regionWidth = currentX - Math.max(CONFIG.LABEL_WIDTH, stableStartX);
                        if (regionWidth > 30) {  // Only draw if wide enough
                            const hexStr = '0x' + stableValue.toString(16).toUpperCase().padStart(
                                Math.ceil(bits.length / 4), '0');
                            this.ctx.fillStyle = COLORS.hex;
                            this.ctx.font = `${CONFIG.FONT_SIZE}px monospace`;
                            this.ctx.textAlign = 'left';
                            this.ctx.textBaseline = 'middle';
                            this.ctx.fillText(hexStr, 
                                Math.max(CONFIG.LABEL_WIDTH + 2, stableStartX), 
                                trace.y + trace.height / 2);
                        }
                    }
                    
                    // Draw transition marker (vertical line)
                    this.ctx.strokeStyle = COLORS.smearedHatch;
                    this.ctx.lineWidth = 1;
                    this.ctx.beginPath();
                    this.ctx.moveTo(currentX, trace.y + 2);
                    this.ctx.lineTo(currentX, trace.y + trace.height - 2);
                    this.ctx.stroke();
                    
                    // Start new stable region
                    stableStartX = currentX;
                    stableValue = currentValue;
                }
            } else {
                // First sample
                stableStartX = currentX;
                stableValue = currentValue;
            }
            
            previousValue = currentValue;
            previousX = currentX;
        }
        
        // Draw final stable region
        if (stableValue !== null && stableStartX !== null && stableStartX < canvasEndX) {
            const regionWidth = canvasEndX - Math.max(CONFIG.LABEL_WIDTH, stableStartX);
            if (regionWidth > 30) {
                const hexStr = '0x' + stableValue.toString(16).toUpperCase().padStart(
                    Math.ceil(bits.length / 4), '0');
                this.ctx.fillStyle = COLORS.hex;
                this.ctx.font = `${CONFIG.FONT_SIZE}px monospace`;
                this.ctx.textAlign = 'left';
                this.ctx.textBaseline = 'middle';
                this.ctx.fillText(hexStr, 
                    Math.max(CONFIG.LABEL_WIDTH + 2, stableStartX), 
                    trace.y + trace.height / 2);
            }
        }
    }

    renderCursor(sampleBuffer, decoder, layout) {
        if (this.cursorX === null || this.cursorCycle === null) return;
        
        // Draw vertical line
        this.ctx.strokeStyle = '#ffffff';
        this.ctx.lineWidth = 1;
        this.ctx.setLineDash([4, 4]);
        this.ctx.beginPath();
        this.ctx.moveTo(this.cursorX, 0);
        this.ctx.lineTo(this.cursorX, this.canvas.height);
        this.ctx.stroke();
        this.ctx.setLineDash([]);
        
        // Find sample at cursor position
        const samples = sampleBuffer.getSamplesInRange(this.cursorCycle, this.cursorCycle + 1n);
        if (samples.length === 0) return;
        
        const sample = samples[0];
        
        // Display values for each trace - above and to the right
        this.ctx.font = `${CONFIG.CURSOR_VALUE_FONT_SIZE}px monospace`;
        this.ctx.textAlign = 'left';
        this.ctx.textBaseline = 'top';
        
        const valueX = this.cursorX + CONFIG.CURSOR_VALUE_OFFSET_X;

        for (const trace of layout) {
            let valueText = '';
            
            if (trace.type === 'addr_hex') {
                const addr = decoder.decodeAddress(sample.gpios);
                valueText = 'Addr' + ': 0x' + addr.toString(16).toUpperCase().padStart(
                    Math.ceil(decoder.pinMap.addr.length / 4), '0');
            } else if (trace.type === 'data_hex') {
                const data = decoder.decodeData(sample.gpios);
                valueText = 'Data' + ': 0x' + data.toString(16).toUpperCase().padStart(
                    Math.ceil(decoder.pinMap.data.length / 4), '0');
            } else if (trace.type === 'addr_bit') {
                const pinNum = decoder.pinMap.addr[trace.bit];
                const isDriven = decoder.isPinDriven(sample.driven, pinNum);
                const val = isDriven ? decoder.extractBit(sample.gpios, pinNum).toString() : 'High-Z';
                valueText = trace.label + ': ' + val;
            } else if (trace.type === 'data_bit') {
                const pinNum = decoder.pinMap.data[trace.bit];
                const isDriven = decoder.isPinDriven(sample.driven, pinNum);
                const val = isDriven ? decoder.extractBit(sample.gpios, pinNum).toString() : 'High-Z';
                valueText = trace.label + ': ' + val;
            } else if (trace.type === 'control') {
                const pinNum = decoder.pinMap.control[trace.signal];
                const isDriven = decoder.isPinDriven(sample.driven, pinNum);
                const val = isDriven ? decoder.extractBit(sample.gpios, pinNum).toString() : 'High-Z';
                valueText = trace.label + ': ' + val;
            }
            
            if (valueText) {
                // Draw background for readability
                const textWidth = this.ctx.measureText(valueText).width;
                const textHeight = CONFIG.CURSOR_VALUE_FONT_SIZE * CONFIG.CURSOR_VALUE_HEIGHT_MULTIPLIER;

                const valueY = trace.y - (trace.height * CONFIG.CURSOR_VALUE_OFFSET_Y) - textHeight;
                
                this.ctx.fillStyle = CONFIG.CURSOR_VALUE_COLOR;
                this.ctx.fillRect(
                    valueX - CONFIG.CURSOR_VALUE_PADDING_X, 
                    valueY - CONFIG.CURSOR_VALUE_PADDING_Y,  // Start at text position
                    textWidth + (CONFIG.CURSOR_VALUE_PADDING_X * 2), 
                    textHeight + (CONFIG.CURSOR_VALUE_PADDING_Y * 2)
                );
                
                this.ctx.fillStyle = CONFIG.CURSOR_VALUE_TEXT_COLOR;
                this.ctx.fillText(valueText, valueX, valueY + 2 * CONFIG.CURSOR_VALUE_PADDING_Y);
            }
        }
    }
}

// =============================================================================
// ANALYZER CONTROLLER
// =============================================================================

class AnalyzerController {
    constructor() {
        this.wasm = new WASMModule();
        this.samples = new SampleBuffer();
        this.decoder = null;
        this.renderer = null;
        this.execution = null;
        
        this.renderTimer = null;
    }
    
    async init() {
        // Initialize WASM
        await this.wasm.init();
        
        // Initialize One ROM
        const result = this.wasm.oneromInit();
        if (!result) {
            throw new Error('Failed to initialize One ROM emulator');
        }
        
        // Set up components
        this.decoder = new SignalDecoder(this.wasm);
        this.execution = new ExecutionEngine(this.wasm, this.samples);
        
        const canvas = document.getElementById('waveform');
        this.renderer = new WaveformRenderer(canvas);
        this.renderer.resize();
        
        // Initial pin map
        const addrBits = parseInt(document.getElementById('addrBits').value);
        this.decoder.buildPinMap(addrBits);
        
        // Set up UI event handlers
        this.setupEventHandlers();
        
        // Start render loop
        this.startRenderLoop();
        
        this.updateStatus('Ready');
    }
    
    setupEventHandlers() {
        // Start button
        document.getElementById('startBtn').addEventListener('click', () => {
            this.startExecution();
        });
        
        // Pause button
        document.getElementById('pauseBtn').addEventListener('click', () => {
            if (this.execution.isPaused()) {
                this.execution.resume();
                this.updateExecutionButtons();
            } else {
                this.execution.pause();
                this.updateExecutionButtons();
            }
        });
        
        // Stop button
        document.getElementById('stopBtn').addEventListener('click', () => {
            this.execution.stop();
            this.updateExecutionButtons();
            this.updateStatus('Stopped');
        });
        
        // Clear button
        document.getElementById('clearBtn').addEventListener('click', () => {
            this.execution.stop();
            this.samples.clear();
            this.wasm.epioResetCycleCount();
            this.renderer.scrollPos = 0;
            this.updateDisplay();
            this.updateExecutionButtons();
            this.updateStatus('Cleared');
        });
        
        // Address bits change
        document.getElementById('addrBits').addEventListener('change', (e) => {
            const bits = parseInt(e.target.value);
            this.decoder.buildPinMap(bits);
            this.updateDisplay();
        });
        
        // Speed control
        document.getElementById('speedControl').addEventListener('change', (e) => {
            this.execution.cyclesPerFrame = parseInt(e.target.value);
        });
        
        // Cycles per pixel slider
        const slider = document.getElementById('cyclesPerPixel');
        const display = document.getElementById('cyclesPerPixelValue');
        slider.addEventListener('input', (e) => {
            const sliderValue = parseFloat(e.target.value);
            
            // Calculate new cycles per pixel
            const oldCyclesPerPixel = this.renderer.cyclesPerPixel;
            const newCyclesPerPixel = this.sliderToCyclesPerPixel(sliderValue);
            
            // Adjust scroll position to keep center of view stable
            const canvasWidth = this.renderer.canvas.width - CONFIG.LABEL_WIDTH;
            const centerCycle = (this.renderer.scrollPos + canvasWidth / 2) * oldCyclesPerPixel;
            this.renderer.scrollPos = Math.max(0, (centerCycle / newCyclesPerPixel) - canvasWidth / 2);
            
            this.renderer.cyclesPerPixel = newCyclesPerPixel;
            display.textContent = this.renderer.cyclesPerPixel.toFixed(2);
        });
        
        // Auto-scroll toggle
        document.getElementById('autoScroll').addEventListener('change', (e) => {
            this.renderer.autoScroll = e.target.checked;
        });
        
        // Scrollbar
        const scrollbar = document.getElementById('scrollbar');
        scrollbar.addEventListener('input', (e) => {
            this.renderer.scrollPos = parseFloat(e.target.value);
            
            // If scrolled to near the end (within 0.1% of max), re-enable auto-scroll
            const scrollRange = parseFloat(scrollbar.max) - parseFloat(scrollbar.min);
            const distanceFromEnd = parseFloat(scrollbar.max) - parseFloat(e.target.value);
            
            if (distanceFromEnd < scrollRange * 0.001) {
                // Near the end - re-enable auto-scroll
                this.renderer.autoScroll = true;
                document.getElementById('autoScroll').checked = true;
            } else {
                // Not at end - disable auto-scroll
                this.renderer.autoScroll = false;
                document.getElementById('autoScroll').checked = false;
            }
        });
        
        // Window resize
        window.addEventListener('resize', () => {
            this.renderer.resize();
        });
        
        // Signal group toggles
        document.getElementById('toggleAddr').addEventListener('change', (e) => {
            this.renderer.showAddr = e.target.checked;
        });
        
        document.getElementById('toggleData').addEventListener('change', (e) => {
            this.renderer.showData = e.target.checked;
        });
        
        document.getElementById('toggleControl').addEventListener('change', (e) => {
            this.renderer.showControl = e.target.checked;
        });
        
        // Collapse/expand buttons
        document.getElementById('collapseAddr').addEventListener('click', (e) => {
            this.renderer.addrExpanded = !this.renderer.addrExpanded;
            e.target.textContent = this.renderer.addrExpanded ? '−' : '+';
        });
        
        document.getElementById('collapseData').addEventListener('click', (e) => {
            this.renderer.dataExpanded = !this.renderer.dataExpanded;
            e.target.textContent = this.renderer.dataExpanded ? '−' : '+';
        });

        // Cursor tracking on canvas
        this.renderer.canvas.addEventListener('mousemove', (e) => {
            const rect = this.renderer.canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;
            
            // Only show cursor if over waveform area (not labels)
            if (x > CONFIG.LABEL_WIDTH) {
                this.renderer.cursorX = x;
                // Calculate which cycle the cursor is over
                const cycleOffset = (x - CONFIG.LABEL_WIDTH + this.renderer.scrollPos) * this.renderer.cyclesPerPixel;
                this.renderer.cursorCycle = BigInt(Math.floor(cycleOffset));
            } else {
                this.renderer.cursorX = null;
                this.renderer.cursorCycle = null;
            }
        });

        this.renderer.canvas.addEventListener('mouseleave', () => {
            this.renderer.cursorX = null;
            this.renderer.cursorCycle = null;
        });
    }
    
    startExecution() {
        const addrBits = parseInt(document.getElementById('addrBits').value);
        
        this.samples.clear();
        this.wasm.epioResetCycleCount();
        this.renderer.scrollPos = 0;
        
        this.execution.startCompleteRead(addrBits);  // Only pass addrBits
        this.updateExecutionButtons();
        this.updateStatus('Running');
    }
    
    updateExecutionButtons() {
        const startBtn = document.getElementById('startBtn');
        const pauseBtn = document.getElementById('pauseBtn');
        const stopBtn = document.getElementById('stopBtn');
        
        if (this.execution.isRunning()) {
            startBtn.disabled = true;
            pauseBtn.disabled = false;
            stopBtn.disabled = false;
            pauseBtn.textContent = this.execution.isPaused() ? 'Resume' : 'Pause';
        } else {
            startBtn.disabled = false;
            pauseBtn.disabled = true;
            stopBtn.disabled = true;
            pauseBtn.textContent = 'Pause';
        }
    }
    
    // Convert slider value to cycles per pixel (logarithmic scale)
    sliderToCyclesPerPixel(sliderValue) {
        const minLog = Math.log10(CONFIG.CYCLES_PER_PIXEL_MIN);
        const maxLog = Math.log10(CONFIG.CYCLES_PER_PIXEL_MAX);
        const range = maxLog - minLog;
        
        const sliderRange = CONFIG.SLIDER_MAX - CONFIG.SLIDER_MIN;
        const normalized = (sliderValue - CONFIG.SLIDER_MIN) / sliderRange;
        
        const logValue = minLog + normalized * range;
        return Math.pow(10, logValue);
    }
    
    // Continuous render loop
    startRenderLoop() {
        const render = () => {
            if (this.samples.length > 0) {
                this.renderer.updateAutoScroll(this.samples.lastCycle);
            }
            this.updateDisplay();
            
            if (this.execution.isRunning()) {
                this.updateStatus('Running');
            } else if (this.samples.length > 0) {
                if (this.execution.getProgress() === 100) {
                    this.updateStatus('Complete');
                } else {
                    this.updateStatus('Stopped');
                }
            }

            // Always update progress display
            if (this.samples.length > 0) {
                document.getElementById('progress').textContent = 
                    this.execution.getProgress() + '%';
            }
            
            this.renderTimer = requestAnimationFrame(render);
        };
        render();
    }
    
    updateDisplay() {
        this.renderer.render(this.samples, this.decoder);
        this.updateSampleCount();
        this.updateScrollbar();
    }
    
    updateStatus(status) {
        document.getElementById('status').textContent = status;
    }
    
    updateSampleCount() {
        document.getElementById('sampleCount').textContent = this.samples.length;
        document.getElementById('cycleCount').textContent = this.samples.lastCycle.toString();
    }
    
    updateScrollbar() {
        const scrollbar = document.getElementById('scrollbar');
        const canvasWidth = this.renderer.canvas.width - CONFIG.LABEL_WIDTH;
        const maxScroll = Math.max(0, 
            Number(this.samples.lastCycle) / this.renderer.cyclesPerPixel - canvasWidth);
        
        scrollbar.max = maxScroll;
        if (this.renderer.autoScroll) {
            scrollbar.value = this.renderer.scrollPos;
        }
    }
}

// =============================================================================
// INITIALIZATION
// =============================================================================

let analyzer;

window.addEventListener('load', async () => {
    try {
        analyzer = new AnalyzerController();
        await analyzer.init();
    } catch (error) {
        console.error('Failed to initialize analyzer:', error);
        document.getElementById('status').textContent = 'Error: ' + error.message;
    }
});