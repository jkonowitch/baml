// Interface blocks 
class Interface1 {
  prop string
  prop2 int
  propArray string[]
  boolean bool
}

class InterfaceTwo {
  interface string
  prop2 string
}

class InterfaceThree {
  field InterfaceTwo
  another_fiel string
}

class InterfaceFour {
  field InterfaceThree
  another_field string
  images image[]
  single_image image
}
