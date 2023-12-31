const VF = Vex.Flow;

const NOTE_ACC_REGEX = /[A-G](#{1,2}|b{1,2}|n)?/;

/**
 * getAccidental("C/4") === undefined;
 * getAccidental("C##/4") === "##";
 * getAccidental("Dn/5")=== "n";
 */
function getAccidental(noteName) {
  const [_, acc] = noteName.match(NOTE_ACC_REGEX);
  return acc;
}

export class Staff {
  constructor(parentEl, width, height, noteName) {
    this.renderer = new VF.Renderer(parentEl, VF.Renderer.Backends.SVG);
    this.renderer.resize(width, height);
    this.context = this.renderer.getContext();

    const stave = new VF.Stave(0, 0, width-1)
      .setContext(this.context)
      .addClef("treble")
      .draw();

    if (noteName) {
      const note = new VF.StaveNote({
        keys: [noteName],
        duration: "w",
        align_center: true,
      }).setStave(stave);

      const acc = getAccidental(noteName);
      if (acc) {
        note.addModifier(new VF.Accidental(acc));
      }

      this.noteGroup = this.context.openGroup();
      VF.Formatter.FormatAndDraw(this.context, stave, [note]);
      this.context.closeGroup();
    }
  }

  clear() {
    if (this.noteGroup) {
      this.context.svg.removeChild(this.noteGroup);
      this.noteGroup = null;
    }
  }
}
