import Asciidoctor from "@asciidoctor/core";

class Instance {
  static instance = new Asciidoctor();
  static convert(content, options) {
    return Instance.instance.convert(content, options);
  }
}

export { Instance as Asciidoctor };