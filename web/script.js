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

const defuseCodeInput = (function () {
    let defusePromiseResolve = null
    document.getElementById('defuse-form').addEventListener('submit', event => {
        event.preventDefault();
        if (defusePromiseResolve != null) {
            defusePromiseResolve(parseInt(document.getElementById('defuse-input').value))
        }
    })
    return function () {
        return new Promise(resolve => {
            defusePromiseResolve = resolve
        })
    }
})()

function wait(ms) {
    return new Promise(resolve => {
        setTimeout(() => resolve(), ms);
    })
}

async function connectRtc() {
    const connection = new RTCPeerConnection();

}

async function monitorTouch() {
    const toleranceRadius = 75;
    const svgNs = "http://www.w3.org/2000/svg";
    const touchArea = document.createElementNS(svgNs, "svg");
    touchArea.classList.add('full')
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
    let out = await onAlertTrigger(42);
    console.log('alert cancel:', out)
    //await connectRtc();
}

function promptDefuse() {
    document.getElementById('defuse').classList.remove('disable')
    setTimeout(() => document.getElementById('defuse-input').focus())
}

async function onAlertTrigger(code) {
    promptDefuse()
    const codeInput = defuseCodeInput()
    const timeout1 = wait(2000)
    const timeout2 = wait(4000)
    const result1 = await Promise.race([codeInput, timeout1]);
    if (result1 === undefined) {
        document.getElementById('defuse').style.background = 'black'
        //timeout
        const result2 = await Promise.race([codeInput, timeout2]).finally(() => {
            document.getElementById('defuse').style.background = undefined
        });
        return result2 === code
    } else {
        return result1 === code
    }
}

main().then(() => {
    console.log('main resolved')
})

console.log('sync')