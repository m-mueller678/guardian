async function monitorAccelerometer(threshold) {
    const acl = new Accelerometer({frequency: 60.0})
    await new Promise(resolve => {
        acl.addEventListener('reading', () => {
            const abs = acl.x * acl.x + acl.y * acl.y + acl.z + acl.z
            if (abs > threshold) {
                resolve()
            }
        })
        acl.start()
    })
}

async function connectRtc() {
    const connection = new RTCPeerConnection();

}

async function monitorTouch() {
    const toleranceRadius = 75;
    const svgNs = "http://www.w3.org/2000/svg";
    const touchArea = document.createElementNS(svgNs, "svg");
    touchArea.id = 'touch-area'
    document.body.appendChild(touchArea)
    let anyTouches = false;
    let listeners = null;
    let touchX = 0;
    let touchY = 0;
    const area = touchArea.getBoundingClientRect()
    console.log(area)
    await new Promise(resolve => {
        document.getElementsByTagName('html')[0].classList.add('noscroll')
        touchArea.addEventListener('touchstart', e => {
            if (anyTouches) {
                resolve()
                return;
            }
            anyTouches = true
            const touch = e.touches[0]
            touchX = touch.clientX
            touchY = touch.clientY
            console.log(touch)
            const inner = document.createElementNS(svgNs, 'circle')
            inner.setAttributeNS(null, 'cx', touchX - area.x)
            inner.setAttributeNS(null, 'cy', touchY - area.y)
            inner.setAttributeNS(null, 'r', toleranceRadius)
            inner.setAttributeNS(null, 'fill', '#00ff13')
            const outer = document.createElementNS(svgNs, 'circle')
            outer.setAttributeNS(null, 'cx', touchX - area.x)
            outer.setAttributeNS(null, 'cy', touchY - area.y)
            outer.setAttributeNS(null, 'r', 300)
            outer.setAttributeNS(null, 'fill', '#066a0d')
            touchArea.appendChild(outer)
            touchArea.appendChild(inner)
        })
        touchArea.addEventListener('touchend', e => {
            resolve()
        })
        touchArea.addEventListener('touchcancel', e => {
            resolve()
        })
        touchArea.addEventListener('touchmove', e => {
            const touch = e.touches[0]
            const dx = touchX - touch.clientX
            const dy = touchY - touch.clientY
            if (dx * dx + dy * dy > toleranceRadius * toleranceRadius) {
                resolve()
            }
        })
    }).finally(() => {
        document.getElementsByTagName('html')[0].classList.remove('noscroll')
        touchArea.remove()
    })
}

async function main() {
    await Promise.race([monitorTouch()])
    document.body.style.background = 'red'
    await connectRtc();
}


main().then(() => {
    console.log('main resolved')
})

console.log('sync')